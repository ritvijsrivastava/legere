//! Serves archived pages out of an article's ZIM file into the webview via
//! a custom `zim://` URI scheme, registered in `lib.rs`. The frontend
//! builds request URLs with `convertFileSrc('<article_id>/<local_path>',
//! 'zim')` — see `frontend/src/lib/api.ts::zimUrl`.

use std::borrow::Cow;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use lru::LruCache;
use tauri::http::{Response, StatusCode, header};
use wraith_zim::ZimReader;

use crate::db::queries;
use crate::state::AppState;

/// How many articles' ZIM files stay open in memory at once.
/// `ZimReader::open` loads the whole file into RAM, and per-article
/// archives are only ever a few MB, so a small cache is enough to avoid
/// re-reading from disk on every asset request for whichever article is
/// currently open, without holding more than a handful of archives live.
const CACHE_CAPACITY: usize = 4;

/// Per-article-ZIM read cache, shared via [`AppState`]. Also the eviction
/// point when an article is deleted (Phase 3): a stale cached reader must
/// not keep answering requests for a ZIM file that's been removed from
/// disk.
pub struct ZimCache {
    inner: Mutex<LruCache<String, Arc<ZimReader>>>,
}

impl ZimCache {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(LruCache::new(NonZeroUsize::new(CACHE_CAPACITY).unwrap())),
        }
    }

    /// Drops any cached reader for `article_id` — both the content-zim and
    /// full-archive-zim entries, since an article can have either or both
    /// live in the cache at once (see `serve`'s two-tier lookup). Called
    /// on article delete, re-capture, archive download, and archive
    /// eviction — all of which replace or remove an underlying ZIM file
    /// out from under whatever's cached.
    pub fn evict(&self, article_id: &str) {
        let mut cache = self.inner.lock().unwrap();
        cache.pop(&content_cache_key(article_id));
        cache.pop(&archive_cache_key(article_id));
    }

    fn get_or_open(
        &self,
        article_id: &str,
        zim_abs_path: &std::path::Path,
    ) -> Result<Arc<ZimReader>, wraith_zim::ZimError> {
        let mut cache = self.inner.lock().unwrap();
        if let Some(reader) = cache.get(article_id) {
            return Ok(reader.clone());
        }
        let reader = Arc::new(ZimReader::open(zim_abs_path)?);
        cache.put(article_id.to_string(), reader.clone());
        Ok(reader)
    }
}

impl Default for ZimCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Distinct cache keys per article for its two possible ZIM files — a
/// plain `article_id` key would collide two entirely different files
/// (content zim vs full archive) onto one `ZimCache` slot.
fn content_cache_key(article_id: &str) -> String {
    format!("{article_id}:content")
}
fn archive_cache_key(article_id: &str) -> String {
    format!("{article_id}:archive")
}

fn text_response(status: StatusCode, body: &'static str) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Cow::Borrowed(body.as_bytes()))
        .expect("static response should always build")
}

/// Parses `<article_id>/<entry_path...>` out of a request path (the
/// leading `/` already stripped by the caller).
///
/// Decodes the *whole* path first, then splits — not the other way
/// around. `convertFileSrc(filePath, protocol)` (see `frontend/src/lib/
/// api.ts::zimUrl`) runs `encodeURIComponent` over the entire joined
/// `<article_id>/<local_path>` string as one unit, which percent-encodes
/// every internal `/` as `%2F` too — confirmed against Tauri's own
/// `scripts/core.js`. Splitting on a literal `/` before decoding would
/// therefore never find the boundary at all when slashes arrive still
/// encoded; decoding first is correct either way, since decoding an
/// already-literal `/` is a no-op.
fn parse_request_path(path: &str) -> Option<(String, String)> {
    let decoded = percent_encoding::percent_decode_str(path.trim_start_matches('/'))
        .decode_utf8_lossy()
        .into_owned();
    let (article_id, entry_path) = decoded.split_once('/')?;
    if article_id.is_empty() || entry_path.is_empty() {
        return None;
    }
    Some((article_id.to_string(), entry_path.to_string()))
}

fn ok_response(bytes: Vec<u8>, mimetype: &str) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mimetype)
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .body(Cow::Owned(bytes))
        .expect("response with a validated mimetype should always build")
}

/// Looks `full_path` up in whichever ZIM `rel_path` points at, using
/// `cache_key` for `ZimCache`. Returns `None` (rather than a response) on
/// a clean miss — no such entry in an openable archive — so the caller can
/// fall through to the next tier; an actually-unreadable file still
/// short-circuits straight to a response, since that's not a "try the
/// next tier" situation.
fn try_serve_from(
    state: &AppState,
    cache_key: &str,
    rel_path: &str,
    full_path: &str,
) -> Result<Option<Response<Cow<'static, [u8]>>>, Response<Cow<'static, [u8]>>> {
    let abs_path = state.data_dir.join(rel_path);
    let reader = state
        .zim_cache
        .get_or_open(cache_key, &abs_path)
        .map_err(|_| text_response(StatusCode::NOT_FOUND, "archive unreadable"))?;
    match reader.get_entry_by_full_path(full_path) {
        Ok(Some((bytes, mimetype))) => Ok(Some(ok_response(bytes, &mimetype))),
        Ok(None) => Ok(None),
        Err(_) => Err(text_response(StatusCode::INTERNAL_SERVER_ERROR, "archive read error")),
    }
}

/// Handles one `zim://` request: validates `article_id` against the
/// database (this is the traversal guard — see
/// `queries::get_article_zim_paths`'s doc comment), then resolves
/// `entry_path` against whichever of the article's two ZIM files actually
/// has it.
///
/// Two-tier lookup, in this order: the persistent content zim first (every
/// image the readable view references lives there, and it's never
/// evicted), then the full archive if one happens to be cached locally
/// (the main page, and everything a `local_legacy` row's readable view
/// referenced before content zims existed, since that row's `content_zim_path`
/// is `NULL` and its one archive serves both roles). This keeps readable-view
/// images available regardless of whether the full archive is currently
/// cached — the whole reason the two are separate files.
pub fn serve(state: &AppState, request_path: &str) -> Response<Cow<'static, [u8]>> {
    let Some((article_id, entry_path)) = parse_request_path(request_path) else {
        return text_response(StatusCode::BAD_REQUEST, "malformed zim:// request path");
    };

    let conn = match state.pool.get() {
        Ok(conn) => conn,
        Err(_) => return text_response(StatusCode::INTERNAL_SERVER_ERROR, "database unavailable"),
    };
    let paths = match queries::get_article_zim_paths(&conn, &article_id) {
        Ok(Some(paths)) => paths,
        Ok(None) => return text_response(StatusCode::NOT_FOUND, "no such article"),
        Err(_) => return text_response(StatusCode::INTERNAL_SERVER_ERROR, "database error"),
    };
    drop(conn);

    let full_path = format!("C{entry_path}");

    if let Some(content_zim_path) = &paths.content_zim_path {
        match try_serve_from(state, &content_cache_key(&article_id), content_zim_path, &full_path) {
            Ok(Some(response)) => return response,
            Ok(None) => {} // not in the content zim — fall through to the archive
            Err(response) => return response,
        }
    }

    if let Some(zim_path) = &paths.zim_path {
        match try_serve_from(state, &archive_cache_key(&article_id), zim_path, &full_path) {
            Ok(Some(response)) => return response,
            Ok(None) => return text_response(StatusCode::NOT_FOUND, "no such entry in archive"),
            Err(response) => return response,
        }
    }

    text_response(StatusCode::NOT_FOUND, "no such entry in archive")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use tokio::sync::Mutex as TokioMutex;
    use wraith_zim::ZimWriter;

    /// Writes `zim_writer`'s archive to disk and inserts the matching
    /// `articles` row into `state`'s own DB — the multi-article half of
    /// test setup, factored out so a single test can populate more than
    /// one article against a shared `AppState`/`ZimCache` (see the LRU
    /// capacity test below).
    fn insert_article(
        state: &AppState,
        data_dir: &std::path::Path,
        article_id: &str,
        zim_writer: ZimWriter,
    ) {
        let archives_dir = data_dir.join("archives");
        std::fs::create_dir_all(&archives_dir).expect("mkdir archives");
        let zim_rel_path = format!("archives/{article_id}.zim");
        zim_writer
            .write(&data_dir.join(&zim_rel_path))
            .expect("write test zim");

        let conn = state.pool.get().expect("get conn");
        conn.execute(
            "INSERT INTO articles (
                id, source_name, source_type, title, link, excerpt,
                content_html, fetched_at, zim_path, zim_main_path, updated_at
            ) VALUES (?1, 'Direct link', 'direct', 'Test', ?3, 'x',
                      '<p>x</p>', '2026-01-01T00:00:00Z', ?2, 'index.html', '2026-01-01T00:00:00Z')",
            rusqlite::params![article_id, zim_rel_path, format!("https://example.com/{article_id}")],
        )
        .expect("insert test article");
    }

    fn build_state_with_article(
        data_dir: &std::path::Path,
        article_id: &str,
        zim_writer: ZimWriter,
    ) -> AppState {
        let db_path = data_dir.join("legere.db");
        let pool = db::build_pool(&db_path).expect("build pool");
        {
            let mut conn = pool.get().expect("get conn");
            db::schema::migrate(&mut conn).expect("migrate");
        }

        let state = AppState {
            pool,
            http_client: reqwest::Client::new(),
            server_http_client: reqwest::Client::new(),
            data_dir: data_dir.to_path_buf(),
            autosync_handle: TokioMutex::new(None),
            zim_cache: ZimCache::new(),
            last_foreground_sync: std::sync::Mutex::new(None),
        };
        insert_article(&state, data_dir, article_id, zim_writer);
        state
    }

    fn sample_writer() -> ZimWriter {
        ZimWriter::new()
            .name("test")
            .title("Test")
            .creator("Legere")
            .publisher("Legere")
            .date("2026-01-01")
            .description("test")
            .language("eng")
            .add_content("index.html", "text/html", "Test", "<p>hello</p>".as_bytes().to_vec())
            .add_content("style.css", "text/css", "", "body{}".as_bytes().to_vec())
            .main_page("index.html")
    }

    /// Writes a content zim (`photo.jpg` only, no page HTML — matching
    /// `capture::archive::write_content_zim`'s real shape) and a separate
    /// full-archive zim (`index.html` main page + its own `photo.jpg`) for
    /// the same article, so the two-tier lookup actually has two distinct
    /// files to choose between rather than falling back on a fixture that
    /// happens to look the same either way.
    fn insert_article_with_content_and_archive_zim(state: &AppState, data_dir: &std::path::Path, article_id: &str) {
        let content_dir = data_dir.join("content");
        let archives_dir = data_dir.join("archives");
        std::fs::create_dir_all(&content_dir).expect("mkdir content");
        std::fs::create_dir_all(&archives_dir).expect("mkdir archives");

        let content_zim_rel = format!("content/{article_id}.zim");
        ZimWriter::new()
            .name(article_id)
            .title(article_id)
            .creator("Legere")
            .publisher("Legere")
            .date("2026-01-01")
            .description(article_id)
            .language("eng")
            .add_content("photo.jpg", "image/jpeg", "", b"content-zim-photo".to_vec())
            .write(&data_dir.join(&content_zim_rel))
            .expect("write test content zim");

        let zim_rel = format!("archives/{article_id}.zim");
        ZimWriter::new()
            .name(article_id)
            .title(article_id)
            .creator("Legere")
            .publisher("Legere")
            .date("2026-01-01")
            .description(article_id)
            .language("eng")
            .add_content("index.html", "text/html", "Test", b"<p>full page</p>".to_vec())
            .add_content("photo.jpg", "image/jpeg", "", b"archive-zim-photo".to_vec())
            .main_page("index.html")
            .write(&data_dir.join(&zim_rel))
            .expect("write test archive zim");

        let conn = state.pool.get().expect("get conn");
        conn.execute(
            "INSERT INTO articles (
                id, source_name, source_type, title, link, excerpt,
                content_html, fetched_at, content_zim_path, zim_path, zim_main_path, updated_at
            ) VALUES (?1, 'Direct link', 'direct', 'Test', ?4, 'x',
                      '<p>x</p>', '2026-01-01T00:00:00Z', ?2, ?3, 'index.html', '2026-01-01T00:00:00Z')",
            rusqlite::params![article_id, content_zim_rel, zim_rel, format!("https://example.com/{article_id}")],
        )
        .expect("insert test article");
    }

    /// Proves the two-tier lookup actually prefers the content zim (not
    /// just falls back to the archive by coincidence): the same
    /// `photo.jpg` path exists in both files with different bytes, and the
    /// content zim's copy — the one the readable view's `legere-zim:`
    /// tokens actually point at — must win.
    #[test]
    fn prefers_the_content_zim_over_the_archive_for_a_shared_path() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let state = build_state_with_article(data_dir.path(), "unused", sample_writer());
        insert_article_with_content_and_archive_zim(&state, data_dir.path(), "article-both");

        let resp = serve(&state, "/article-both/photo.jpg");
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.body().as_ref(), b"content-zim-photo");
    }

    /// A path only the archive has (the main page — content zims never
    /// contain one) must fall through to it once the content zim reports a
    /// clean miss, rather than 404ing at the first tier.
    #[test]
    fn falls_back_to_the_archive_zim_when_the_content_zim_lacks_the_entry() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let state = build_state_with_article(data_dir.path(), "unused", sample_writer());
        insert_article_with_content_and_archive_zim(&state, data_dir.path(), "article-both");

        let resp = serve(&state, "/article-both/index.html");
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.body().as_ref(), b"<p>full page</p>");
    }

    /// A `local_legacy`-style row (`content_zim_path` is `NULL`, only
    /// `zim_path` set — exactly what the V3 migration produces for every
    /// pre-remodel article) must resolve entirely via the archive-zim
    /// fallback, with no content-zim tier to even attempt.
    #[test]
    fn resolves_entirely_via_the_archive_zim_when_there_is_no_content_zim() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let state = build_state_with_article(data_dir.path(), "legacy-article", sample_writer());

        let resp = serve(&state, "/legacy-article/style.css");
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.body().as_ref(), b"body{}");
    }

    #[test]
    fn serves_the_main_page() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let state = build_state_with_article(data_dir.path(), "article-1", sample_writer());

        let resp = serve(&state, "/article-1/index.html");
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get(header::CONTENT_TYPE).unwrap(), "text/html");
        assert_eq!(resp.body().as_ref(), b"<p>hello</p>");
    }

    #[test]
    fn serves_an_entry_via_the_percent_encoded_form_convert_file_src_actually_sends() {
        // `convertFileSrc('article-1/https/example.com/article.html', 'zim')`
        // runs `encodeURIComponent` over the *entire* joined string as one
        // unit (confirmed against Tauri's own scripts/core.js), so every
        // `/` in it — including the ones separating the article id from
        // the nested archive path — arrives percent-encoded as `%2F`, not
        // as a literal slash. This is the real request shape; the other
        // tests in this module use literal slashes only because `serve`
        // must handle both correctly (decoding first makes a literal `/`
        // a no-op), and this test is what actually proves that.
        let data_dir = tempfile::tempdir().expect("tempdir");
        let writer = ZimWriter::new()
            .name("test")
            .title("Test")
            .creator("Legere")
            .publisher("Legere")
            .date("2026-01-01")
            .description("test")
            .language("eng")
            .add_content(
                "https/example.com/article.html",
                "text/html",
                "Test",
                "<p>nested</p>".as_bytes().to_vec(),
            )
            .main_page("https/example.com/article.html");
        let state = build_state_with_article(data_dir.path(), "article-1", writer);

        let resp = serve(&state, "/article-1%2Fhttps%2Fexample.com%2Farticle.html");
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.body().as_ref(), b"<p>nested</p>");
    }

    #[test]
    fn serves_a_secondary_asset_with_its_own_mimetype() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let state = build_state_with_article(data_dir.path(), "article-1", sample_writer());

        let resp = serve(&state, "/article-1/style.css");
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get(header::CONTENT_TYPE).unwrap(), "text/css");
        assert_eq!(resp.body().as_ref(), b"body{}");
    }

    #[test]
    fn returns_404_for_an_entry_not_in_the_archive() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let state = build_state_with_article(data_dir.path(), "article-1", sample_writer());

        let resp = serve(&state, "/article-1/does-not-exist.html");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn returns_404_for_an_unknown_article_id_even_if_a_matching_zim_file_exists_on_disk() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        // Deliberately write a ZIM at the path an id *would* map to,
        // without ever inserting the corresponding `articles` row — this
        // is the exact traversal/forged-id scenario the DB lookup guards
        // against.
        let state = build_state_with_article(data_dir.path(), "real-article", sample_writer());
        std::fs::create_dir_all(data_dir.path().join("archives")).ok();
        sample_writer()
            .write(&data_dir.path().join("archives/not-a-real-article.zim"))
            .expect("write rogue zim");

        let resp = serve(&state, "/not-a-real-article/index.html");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn rejects_a_request_path_with_no_entry_segment() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let state = build_state_with_article(data_dir.path(), "article-1", sample_writer());

        let resp = serve(&state, "/article-1");
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn cache_serves_repeated_requests_without_reopening_the_file() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let state = build_state_with_article(data_dir.path(), "article-1", sample_writer());

        let first = serve(&state, "/article-1/index.html");
        assert_eq!(first.status(), StatusCode::OK);

        // Corrupt the on-disk file; a cache hit must not need to re-read
        // it, so this must still succeed.
        std::fs::write(
            data_dir.path().join("archives/article-1.zim"),
            b"not a zim file",
        )
        .unwrap();

        let second = serve(&state, "/article-1/index.html");
        assert_eq!(second.status(), StatusCode::OK);
        assert_eq!(second.body().as_ref(), b"<p>hello</p>");
    }

    /// Proves `ZimCache`'s bounded-memory claim (module doc: "without
    /// holding more than a handful of archives live") is actually
    /// enforced, not just configured — filling the cache past
    /// `CACHE_CAPACITY` must evict the least-recently-used entry on its
    /// own, the same way `evict_forces_a_fresh_read_from_disk` proves for
    /// *explicit* eviction.
    #[test]
    fn cache_evicts_the_least_recently_used_entry_once_over_capacity() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let state = build_state_with_article(data_dir.path(), "article-0", sample_writer());
        for i in 1..CACHE_CAPACITY {
            insert_article(&state, data_dir.path(), &format!("article-{i}"), sample_writer());
        }

        // Fill the cache to exactly its capacity, oldest (article-0) first.
        for i in 0..CACHE_CAPACITY {
            let resp = serve(&state, &format!("/article-{i}/index.html"));
            assert_eq!(resp.status(), StatusCode::OK);
        }

        // One more distinct article pushes the cache over capacity —
        // article-0 was the least recently used (touched first, never
        // again), so it should be the one evicted.
        insert_article(&state, data_dir.path(), "article-overflow", sample_writer());
        let resp = serve(&state, "/article-overflow/index.html");
        assert_eq!(resp.status(), StatusCode::OK);

        // If article-0 is still cached, this corrupted file would never be
        // read and the request would still succeed. It doesn't — the
        // capacity eviction, not an explicit `evict()` call, is what forced
        // the fresh (failing) read.
        std::fs::write(
            data_dir.path().join("archives/article-0.zim"),
            b"not a zim file",
        )
        .unwrap();
        let resp = serve(&state, "/article-0/index.html");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn evict_forces_a_fresh_read_from_disk() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let state = build_state_with_article(data_dir.path(), "article-1", sample_writer());

        let first = serve(&state, "/article-1/index.html");
        assert_eq!(first.status(), StatusCode::OK);

        state.zim_cache.evict("article-1");
        std::fs::write(
            data_dir.path().join("archives/article-1.zim"),
            b"not a zim file",
        )
        .unwrap();

        let second = serve(&state, "/article-1/index.html");
        assert_eq!(second.status(), StatusCode::NOT_FOUND);
    }
}
