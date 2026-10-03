use thiserror::Error;

use crate::capture;
use crate::db::queries;
use crate::models::Source;
use crate::state::AppState;

#[derive(Debug, Error)]
pub enum RssSyncError {
    #[error("failed to fetch feed: {0}")]
    Fetch(#[from] reqwest::Error),
    #[error("failed to parse feed: {0}")]
    Parse(#[from] feed_rs::parser::ParseFeedError),
    #[error(transparent)]
    Db(#[from] rusqlite::Error),
    #[error(transparent)]
    Pool(#[from] r2d2::Error),
}

/// Bounds how many entries a single sync processes, so a feed's full
/// backlog doesn't get captured (with a real network fetch per entry) the
/// first time a long-lived feed is added.
const MAX_ENTRIES_PER_SYNC: usize = 30;

/// Fetches `source`'s feed, skips entries already stored (by link), and
/// locally captures every new one via the shared
/// [`capture::capture_local`] pipeline. Returns the number of newly
/// captured articles.
pub async fn sync_rss_source(state: &AppState, source: &Source) -> Result<u32, RssSyncError> {
    let feed_url = source.feed_url.as_deref().unwrap_or_default();

    // Conditional GET: most feeds haven't changed since the last sync
    // (hourly autosync against a slow-moving blog is the common case), so
    // sending back whatever `ETag`/`Last-Modified` the last successful
    // fetch returned lets a well-behaved server answer with a bodyless
    // `304 Not Modified` instead of the full feed again. Either header
    // (or both, or neither) may be absent — a server that ignores them
    // entirely just always returns `200` with a full body, exactly like
    // before this existed.
    let mut request = state.http_client.get(feed_url);
    if let Some(etag) = source.feed_etag.as_deref() {
        request = request.header(reqwest::header::IF_NONE_MATCH, etag);
    }
    if let Some(last_modified) = source.feed_last_modified.as_deref() {
        request = request.header(reqwest::header::IF_MODIFIED_SINCE, last_modified);
    }
    let response = request.send().await?;

    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(0);
    }

    let etag = header_str(&response, reqwest::header::ETAG);
    let last_modified = header_str(&response, reqwest::header::LAST_MODIFIED);
    let bytes = response.bytes().await?;
    let feed = feed_rs::parser::parse(&bytes[..])?;

    {
        let conn = state.pool.get()?;
        queries::update_source_feed_cache(
            &conn,
            &source.id,
            etag.as_deref(),
            last_modified.as_deref(),
        )?;
    }

    if let Some(title) = feed
        .title
        .as_ref()
        .map(|t| t.content.trim())
        .filter(|t| !t.is_empty())
    {
        let conn = state.pool.get()?;
        queries::set_source_name_if_default(&conn, &source.id, title, feed_url)?;
    }

    let mut new_count = 0u32;
    for entry in feed.entries.into_iter().take(MAX_ENTRIES_PER_SYNC) {
        let Some(link) = entry.links.first().map(|l| l.href.clone()) else {
            continue;
        };
        let tags: Vec<String> = entry.categories.iter().map(|c| c.term.clone()).collect();

        let already_exists = {
            let conn = state.pool.get()?;
            queries::article_link_exists(&conn, &link)?
        };
        if already_exists {
            continue;
        }

        let id = uuid::Uuid::new_v4().to_string();
        // Never fails outright: a dead/unparseable entry still comes back
        // as a link-only output (`capture_failed = true`) and gets stored,
        // so it's marked seen (via `UNIQUE(link)`) instead of being
        // re-attempted on every future poll. See
        // `capture::capture_local_or_link_only`.
        let output =
            capture::capture_local_or_link_only(&state.http_client, &state.data_dir, &id, &link)
                .await;
        if output.capture_failed {
            tracing::warn!(
                source_id = %source.id,
                %link,
                error = ?output.capture_error,
                "failed to capture RSS entry; stored as link-only"
            );
        }
        let conn = state.pool.get()?;
        let inserted =
            queries::insert_captured_article(&conn, &id, Some(&source.id), "rss", &output, &tags)?;
        if inserted {
            new_count += 1;
        }
    }

    Ok(new_count)
}

fn header_str(response: &reqwest::Response, name: reqwest::header::HeaderName) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use tokio::sync::Mutex;

    use super::*;
    use crate::db;
    use crate::test_support;

    /// Exercises an RSS sync end-to-end against a local fixture feed (no
    /// live network): fetch feed -> parse -> capture the entry -> insert
    /// into SQLite, then a second sync of the same feed should find the
    /// entry already captured (by link) and add nothing new — the dedup
    /// path that keeps autosync from re-fetching a feed's entire backlog
    /// every interval.
    #[tokio::test]
    async fn syncs_a_fixture_feed_and_dedupes_on_second_sync() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let db_path = data_dir.path().join("legere.db");
        let pool = db::build_pool(&db_path).expect("build pool");
        {
            let mut conn = pool.get().expect("get conn");
            db::schema::migrate(&mut conn).expect("migrate");
        }

        let base_url = test_support::spawn().await;
        let state = AppState {
            pool: pool.clone(),
            http_client: test_support::plain_client(),
            update_http_client: test_support::plain_client(),
            data_dir: data_dir.path().to_path_buf(),
            autosync_handle: Mutex::new(None),
            remote_sync_handle: Mutex::new(None),
            last_foreground_sync: std::sync::Mutex::new(None),
            import_cancel: Mutex::new(None),
            article_import_cancel: Mutex::new(None),
            remote_sync_cancel: Mutex::new(None),
            capture_jobs: Default::default(),
            remote_sync_client_cache: Default::default(),
            pending_update: Default::default(),
            pending_linux_update: Default::default(),
        };

        let source = {
            let conn = pool.get().expect("get conn");
            queries::insert_rss_source(&conn, "Fixture Feed", &format!("{base_url}/feed.xml"))
                .expect("insert source")
        };

        let first_sync_count = sync_rss_source(&state, &source)
            .await
            .expect("first sync should succeed against the fixture feed");
        assert_eq!(first_sync_count, 1, "fixture feed has exactly one entry");

        let second_sync_count = sync_rss_source(&state, &source)
            .await
            .expect("second sync should also succeed");
        assert_eq!(
            second_sync_count, 0,
            "already-captured entries must not be recaptured"
        );

        let conn = pool.get().expect("get conn");
        let articles = queries::list_articles(&conn).expect("list articles");
        assert_eq!(articles.len(), 1);
        assert!(articles.iter().all(|a| a.source_type == "rss"));
    }

    /// An entry whose link can't be captured (404s here) must still be
    /// inserted as a link-only article (marking it seen so it isn't
    /// re-fetched on the next poll) instead of being silently dropped
    /// and retried forever. Uses its own tiny feed server (rather than
    /// `test_support::spawn`'s shared fixture feed, whose one entry always
    /// resolves) with a single entry pointing at a path with no route.
    #[tokio::test]
    async fn a_dead_feed_entry_is_still_saved_as_link_only_and_marked_seen() {
        use axum::Router;
        use axum::http::header::CONTENT_TYPE;
        use axum::routing::get;

        let data_dir = tempfile::tempdir().expect("tempdir");
        let db_path = data_dir.path().join("legere.db");
        let pool = db::build_pool(&db_path).expect("build pool");
        {
            let mut conn = pool.get().expect("get conn");
            db::schema::migrate(&mut conn).expect("migrate");
        }

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("local_addr");
        let base_url = format!("http://localhost:{}", addr.port());
        let feed_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Dead Entry Feed</title>
    <link>{base_url}</link>
    <item>
      <title>Gone</title>
      <link>{base_url}/does-not-exist.html</link>
      <guid>{base_url}/does-not-exist.html</guid>
    </item>
  </channel>
</rss>"#
        );
        let app = Router::new().route(
            "/feed.xml",
            get(move || {
                let body = feed_xml.clone();
                async move { ([(CONTENT_TYPE, "application/rss+xml")], body) }
            }),
        );
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("fixture server");
        });

        let state = AppState {
            pool: pool.clone(),
            http_client: test_support::plain_client(),
            update_http_client: test_support::plain_client(),
            data_dir: data_dir.path().to_path_buf(),
            autosync_handle: Mutex::new(None),
            remote_sync_handle: Mutex::new(None),
            last_foreground_sync: std::sync::Mutex::new(None),
            import_cancel: Mutex::new(None),
            article_import_cancel: Mutex::new(None),
            remote_sync_cancel: Mutex::new(None),
            capture_jobs: Default::default(),
            remote_sync_client_cache: Default::default(),
            pending_update: Default::default(),
            pending_linux_update: Default::default(),
        };

        let source = {
            let conn = pool.get().expect("get conn");
            queries::insert_rss_source(&conn, "Dead Entry Feed", &format!("{base_url}/feed.xml"))
                .expect("insert source")
        };

        let new_count = sync_rss_source(&state, &source)
            .await
            .expect("sync should succeed even though the entry's own fetch fails");
        assert_eq!(
            new_count, 1,
            "the link-only row still counts as newly captured"
        );

        let conn = pool.get().expect("get conn");
        let articles = queries::list_articles(&conn).expect("list articles");
        assert_eq!(articles.len(), 1);
        let detail = queries::get_article(&conn, &articles[0].id)
            .unwrap()
            .unwrap();
        assert!(detail.capture_failed);
        assert!(detail.capture_error.is_some());

        // A second sync must not re-attempt it: `UNIQUE(link)` already
        // marked it seen.
        let second_count = sync_rss_source(&state, &source)
            .await
            .expect("second sync should also succeed");
        assert_eq!(second_count, 0);
    }

    /// `add_source` seeds a new source's `name` with its raw feed URL as a
    /// placeholder (the real title isn't known until the feed is actually
    /// fetched) — the first sync must replace it with the feed's own
    /// title, matching real `add_source` usage rather than the other test
    /// above's fixture, which passes a friendly name up front.
    #[tokio::test]
    async fn first_sync_replaces_the_placeholder_name_with_the_feeds_title() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let db_path = data_dir.path().join("legere.db");
        let pool = db::build_pool(&db_path).expect("build pool");
        {
            let mut conn = pool.get().expect("get conn");
            db::schema::migrate(&mut conn).expect("migrate");
        }

        let base_url = test_support::spawn().await;
        let state = AppState {
            pool: pool.clone(),
            http_client: test_support::plain_client(),
            update_http_client: test_support::plain_client(),
            data_dir: data_dir.path().to_path_buf(),
            autosync_handle: Mutex::new(None),
            remote_sync_handle: Mutex::new(None),
            last_foreground_sync: std::sync::Mutex::new(None),
            import_cancel: Mutex::new(None),
            article_import_cancel: Mutex::new(None),
            remote_sync_cancel: Mutex::new(None),
            capture_jobs: Default::default(),
            remote_sync_client_cache: Default::default(),
            pending_update: Default::default(),
            pending_linux_update: Default::default(),
        };

        let feed_url = format!("{base_url}/feed.xml");
        let source = {
            let conn = pool.get().expect("get conn");
            // Matches how `commands::sources::add_source` actually seeds a
            // new source: name == feed_url until the first sync learns better.
            queries::insert_rss_source(&conn, &feed_url, &feed_url).expect("insert source")
        };
        assert_eq!(source.name, feed_url);

        sync_rss_source(&state, &source)
            .await
            .expect("sync should succeed");

        let conn = pool.get().expect("get conn");
        let renamed = queries::get_source(&conn, &source.id).unwrap().unwrap();
        assert_eq!(renamed.name, "Fixture Feed");
    }
}
