//! Serves an article's own persistent content directory (the images its
//! readable view references) into the webview via a custom
//! `legere-content://` URI scheme, registered in `lib.rs`. The frontend
//! builds request URLs with
//! `convertFileSrc('<article_id>/<local_path>', 'legere-content')` — see
//! `frontend/src/lib/api.ts::resolveContentTokens`.
//!
//! Plain files on disk, not a cache of anything — a request just reads
//! `{data_dir}/content/<article_id>/<entry_path>` directly. No in-memory
//! cache: unlike the ZIM archive this replaced (a single multi-MB file
//! worth keeping parsed in memory across requests), each entry here is
//! already its own small file, and the OS page cache already keeps a
//! recently-read one warm.

use std::borrow::Cow;

use tauri::http::{Response, StatusCode, header};

use crate::db::queries;
use crate::state::AppState;

fn text_response(status: StatusCode, body: &'static str) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Cow::Borrowed(body.as_bytes()))
        .expect("static response should always build")
}

/// Parses `<article_id>/<entry_path...>` out of a request path (the
/// leading `/` already stripped by the caller). Decodes the *whole* path
/// first, then splits — `convertFileSrc(filePath, protocol)` runs
/// `encodeURIComponent` over the entire joined `<article_id>/<local_path>`
/// string as one unit, so every `/` in it (including the one separating
/// the article id from the nested content path) can arrive percent-encoded
/// as `%2F` rather than a literal slash. Decoding first handles both forms
/// correctly, since decoding an already-literal `/` is a no-op.
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

/// Guesses a content image's MIME type from its file extension — every
/// entry under `content/<id>/` came from an `img`/`source`/`video`
/// reference (see `capture::localize`), so this only needs to cover the
/// small set of formats those actually serve in practice. Unknown
/// extensions fall back to a generic binary type rather than guessing
/// wrong.
fn guess_content_type(entry_path: &str) -> &'static str {
    let ext = entry_path
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        _ => "application/octet-stream",
    }
}

/// Handles one `legere-content://` request: validates `article_id` against
/// the database (this is the traversal guard — only an id that's actually
/// a stored article's row resolves to a real file), then reads
/// `entry_path` from that article's own content directory. `entry_path`'s
/// segments are already filesystem-safe (see
/// `urlx::local_path_for`'s sanitization — no `..`, no unsafe
/// characters), but `Path::join` is checked against `content_dir` as
/// defense in depth regardless.
pub fn serve(state: &AppState, request_path: &str) -> Response<Cow<'static, [u8]>> {
    let Some((article_id, entry_path)) = parse_request_path(request_path) else {
        return text_response(
            StatusCode::BAD_REQUEST,
            "malformed legere-content:// request path",
        );
    };

    let conn = match state.pool.get() {
        Ok(conn) => conn,
        Err(_) => return text_response(StatusCode::INTERNAL_SERVER_ERROR, "database unavailable"),
    };
    let exists = match queries::article_exists(&conn, &article_id) {
        Ok(exists) => exists,
        Err(_) => return text_response(StatusCode::INTERNAL_SERVER_ERROR, "database error"),
    };
    drop(conn);

    if !exists {
        return text_response(StatusCode::NOT_FOUND, "no such article");
    }

    let content_dir = state.data_dir.join("content").join(&article_id);
    let path = content_dir.join(&entry_path);
    // A canonicalized `path` must still live under a canonicalized
    // `content_dir` — even though `local_path_for`'s own sanitization
    // already prevents `..` segments from reaching stored tokens, a
    // request path is attacker-influenced input in its own right and
    // shouldn't be trusted just because *legitimate* tokens never contain
    // one. Both sides must be canonicalized: on platforms where the data
    // dir itself sits under a symlink (e.g. macOS's `/var` ->
    // `/private/var`), comparing a canonicalized `path` against a
    // non-canonicalized `content_dir` would reject every legitimate
    // request.
    let Ok(root) = content_dir.canonicalize() else {
        return text_response(StatusCode::NOT_FOUND, "no such entry");
    };
    let Ok(canonical) = path.canonicalize() else {
        return text_response(StatusCode::NOT_FOUND, "no such entry");
    };
    if !canonical.starts_with(&root) {
        return text_response(
            StatusCode::BAD_REQUEST,
            "malformed legere-content:// request path",
        );
    }

    match std::fs::read(&canonical) {
        Ok(bytes) => ok_response(bytes, guess_content_type(&entry_path)),
        Err(_) => text_response(StatusCode::NOT_FOUND, "no such entry"),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::db;
    use tokio::sync::Mutex as TokioMutex;

    fn insert_article(state: &AppState, article_id: &str) {
        let conn = state.pool.get().expect("get conn");
        conn.execute(
            "INSERT INTO articles (
                id, source_name, source_type, title, link, excerpt,
                content_html, fetched_at, updated_at
            ) VALUES (?1, 'Direct link', 'direct', 'Test', ?2, 'x',
                      '<p>x</p>', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            rusqlite::params![article_id, format!("https://example.com/{article_id}")],
        )
        .expect("insert test article");
    }

    fn build_state(data_dir: &Path) -> AppState {
        let db_path = data_dir.join("legere.db");
        let pool = db::build_pool(&db_path).expect("build pool");
        {
            let mut conn = pool.get().expect("get conn");
            db::schema::migrate(&mut conn).expect("migrate");
        }
        AppState {
            pool,
            http_client: reqwest::Client::new(),
            data_dir: data_dir.to_path_buf(),
            autosync_handle: TokioMutex::new(None),
            last_foreground_sync: std::sync::Mutex::new(None),
            import_cancel: TokioMutex::new(None),
            pending_update: Default::default(),
            pending_linux_update: Default::default(),
        }
    }

    fn write_entry(data_dir: &Path, article_id: &str, entry_path: &str, bytes: &[u8]) {
        let path = data_dir.join("content").join(article_id).join(entry_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
    }

    #[test]
    fn serves_an_entry() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let state = build_state(data_dir.path());
        insert_article(&state, "article-1");
        write_entry(
            data_dir.path(),
            "article-1",
            "https/example.com/photo.jpg",
            b"jpeg-bytes",
        );

        let resp = serve(&state, "/article-1/https/example.com/photo.jpg");
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "image/jpeg"
        );
        assert_eq!(resp.body().as_ref(), b"jpeg-bytes");
    }

    #[test]
    fn serves_an_entry_via_the_percent_encoded_form_convert_file_src_actually_sends() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let state = build_state(data_dir.path());
        insert_article(&state, "article-1");
        write_entry(
            data_dir.path(),
            "article-1",
            "https/example.com/article.png",
            b"nested",
        );

        let resp = serve(&state, "/article-1%2Fhttps%2Fexample.com%2Farticle.png");
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.body().as_ref(), b"nested");
    }

    #[test]
    fn returns_404_for_an_entry_not_on_disk() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let state = build_state(data_dir.path());
        insert_article(&state, "article-1");

        let resp = serve(&state, "/article-1/does-not-exist.jpg");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn returns_404_for_an_unknown_article_id_even_if_a_matching_dir_exists_on_disk() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let state = build_state(data_dir.path());
        insert_article(&state, "real-article");
        // Deliberately write a file at the path an id *would* map to,
        // without ever inserting the corresponding `articles` row — this
        // is the exact traversal/forged-id scenario the DB lookup guards
        // against.
        write_entry(data_dir.path(), "not-a-real-article", "photo.jpg", b"rogue");

        let resp = serve(&state, "/not-a-real-article/photo.jpg");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn rejects_a_traversal_attempt_even_for_a_real_article_id() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let state = build_state(data_dir.path());
        insert_article(&state, "article-1");
        // A file that genuinely exists outside this article's content
        // dir — the traversal attempt below tries to reach it via `..`.
        std::fs::create_dir_all(data_dir.path().join("content")).unwrap();
        std::fs::write(data_dir.path().join("content/secret.txt"), b"secret").unwrap();
        write_entry(data_dir.path(), "article-1", "placeholder.jpg", b"x");

        let resp = serve(&state, "/article-1/../secret.txt");
        assert_ne!(resp.status(), StatusCode::OK);
    }

    #[test]
    fn rejects_a_request_path_with_no_entry_segment() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let state = build_state(data_dir.path());
        insert_article(&state, "article-1");

        let resp = serve(&state, "/article-1");
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
