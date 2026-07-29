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

    /// Drops any cached reader for `article_id`, so a subsequent request
    /// (if the file still exists) re-opens it from disk rather than
    /// serving stale in-memory content. Called on article delete and
    /// re-capture — both replace or remove the underlying ZIM file out
    /// from under whatever's cached.
    pub fn evict(&self, article_id: &str) {
        self.inner.lock().unwrap().pop(article_id);
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

/// Handles one `zim://` request: validates `article_id` against the
/// database (this is the traversal guard — see
/// `queries::get_article_zim_path`'s doc comment), opens/reuses its
/// cached [`ZimReader`], and resolves `entry_path` within it.
pub fn serve(state: &AppState, request_path: &str) -> Response<Cow<'static, [u8]>> {
    let Some((article_id, entry_path)) = parse_request_path(request_path) else {
        return text_response(StatusCode::BAD_REQUEST, "malformed zim:// request path");
    };

    let conn = match state.pool.get() {
        Ok(conn) => conn,
        Err(_) => return text_response(StatusCode::INTERNAL_SERVER_ERROR, "database unavailable"),
    };
    let zim_rel_path = match queries::get_article_zim_path(&conn, &article_id) {
        Ok(Some(path)) => path,
        Ok(None) => return text_response(StatusCode::NOT_FOUND, "no such article"),
        Err(_) => return text_response(StatusCode::INTERNAL_SERVER_ERROR, "database error"),
    };
    drop(conn);

    let zim_abs_path = state.data_dir.join(&zim_rel_path);
    let reader = match state.zim_cache.get_or_open(&article_id, &zim_abs_path) {
        Ok(reader) => reader,
        Err(_) => return text_response(StatusCode::NOT_FOUND, "archive unreadable"),
    };

    let full_path = format!("C{entry_path}");
    match reader.get_entry_by_full_path(&full_path) {
        Ok(Some((bytes, mimetype))) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mimetype)
            .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
            .body(Cow::Owned(bytes))
            .expect("response with a validated mimetype should always build"),
        Ok(None) => text_response(StatusCode::NOT_FOUND, "no such entry in archive"),
        Err(_) => text_response(StatusCode::INTERNAL_SERVER_ERROR, "archive read error"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use tokio::sync::Mutex as TokioMutex;
    use wraith_zim::ZimWriter;

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

        let archives_dir = data_dir.join("archives");
        std::fs::create_dir_all(&archives_dir).expect("mkdir archives");
        let zim_rel_path = format!("archives/{article_id}.zim");
        zim_writer
            .write(&data_dir.join(&zim_rel_path))
            .expect("write test zim");

        {
            let conn = pool.get().expect("get conn");
            conn.execute(
                "INSERT INTO articles (
                    id, source_name, source_type, title, link, excerpt,
                    content_html, fetched_at, zim_path, zim_main_path, updated_at
                ) VALUES (?1, 'Direct link', 'direct', 'Test', 'https://example.com/x', 'x',
                          '<p>x</p>', '2026-01-01T00:00:00Z', ?2, 'index.html', '2026-01-01T00:00:00Z')",
                rusqlite::params![article_id, zim_rel_path],
            )
            .expect("insert test article");
        }

        AppState {
            pool,
            http_client: reqwest::Client::new(),
            data_dir: data_dir.to_path_buf(),
            autosync_handle: TokioMutex::new(None),
            zim_cache: ZimCache::new(),
        }
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
