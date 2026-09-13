//! Bulk-imports bookmarks from a Raindrop.io CSV export (Settings ->
//! Import). Only `url`, `tags`, `created`, and `favorite` are read from
//! each row — `note`, `excerpt`, `folder`, `cover`, and `highlights` are
//! intentionally ignored: this app has no highlight/annotation storage or
//! UI (the sidebar's old "Highlights" entry was itself just a stub with
//! no working feature behind it, since removed).
//!
//! Every row goes through the same local-capture pipeline
//! (`capture::capture_local`) as manually adding a direct link, run
//! through a small fixed-size worker pool (`CONCURRENCY`) rather than
//! sequentially (a multi-thousand-row export would otherwise take hours)
//! or fully in parallel (which would hammer every site in the export at
//! once). A single row's failure never aborts the batch — a years-old
//! export is expected to contain plenty of dead links — it's recorded in
//! [`ImportSummary::failed`] instead.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Deserialize;
use url::Url;

use crate::db::queries;
use crate::events::{ImportFailure, ImportFinished, ImportProgress};
use crate::state::AppState;
use crate::urlx::{canonicalize, strip_tracking_params};
use crate::{capture, db};

/// Fixed worker-pool size for concurrent row captures: fast enough that a
/// multi-thousand-row export finishes in a reasonable time, without
/// hammering every site in the export at once.
const CONCURRENCY: usize = 5;

/// How many successful imports accumulate before an
/// [`ImportEvent::LibraryChanged`] fires, so a library screen left open
/// during a long import sees new arrivals periodically rather than only
/// once the whole run finishes. [`run_import`] always fires one more at
/// the very end regardless of this cadence (including for a cancelled
/// run), so nothing imported is ever left unreported.
///
/// Higher than it needs to be for responsiveness alone: each firing used
/// to cost the frontend a full library refetch+re-render
/// (`articlesStore.refresh()` over every article), so this was kept low.
/// Now it's a cheap paginated "fetch page 1, prepend what's new" merge
/// (see `libraryStatsStore`/`ArticleCollection` on the frontend), so a
/// larger batch just means fewer redundant events during a multi-thousand-
/// row import, not a less-live-feeling one.
const LIBRARY_REFRESH_BATCH: u32 = 100;

/// `source_name` stored on every imported article — lets the library UI
/// (which groups/labels by `source_name`) distinguish these from
/// manually-added direct links.
const SOURCE_NAME: &str = "Raindrop import";

/// [`run_import`]'s progress/lifecycle notifications, decoupled from
/// `tauri::AppHandle::emit` so the import loop itself stays unit-testable
/// with a plain no-op callback — the emitting side
/// (`commands::import::import_raindrop_csv`) is what actually maps these
/// onto real frontend events, matching how the rest of this codebase
/// keeps event-emitting command wrappers thin and untested in favor of
/// testing the inner logic directly (see e.g. `sources::rss::sync_rss_source`).
pub enum ImportEvent {
    Started {
        total: u32,
    },
    Progress(ImportProgress),
    /// New articles have landed since the last one of these (or since
    /// `Started`, for the first) — a cue to refetch the library list.
    LibraryChanged,
    Finished(ImportFinished),
}

#[derive(Debug, Deserialize)]
struct RaindropRow {
    #[serde(default)]
    title: String,
    url: String,
    #[serde(default)]
    tags: String,
    created: String,
    #[serde(default)]
    favorite: String,
}

/// Splits Raindrop's comma-separated `tags` cell into a trimmed,
/// non-empty tag list.
fn parse_tags(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

#[derive(Debug, Default, Clone)]
pub struct ImportSummary {
    pub total: u32,
    pub imported: u32,
    pub skipped_duplicate: u32,
    pub failed: Vec<ImportFailure>,
    pub cancelled: bool,
}

enum RowOutcome {
    Imported,
    SkippedDuplicate,
    Failed(ImportFailure),
}

/// Best-effort dedup key for a raw CSV URL: parses it and applies the same
/// tracking-strip/canonicalization used for a stored article's `link`, so
/// the pre-check below catches most already-imported rows before paying
/// for a fetch. Not authoritative — the *actually* fetched/redirected URL
/// can still differ (this is the same tradeoff `article_link_exists`
/// documents at every other call site); [`queries::insert_imported_article`]'s
/// `UNIQUE(link)` index is what makes a resulting duplicate insert safe
/// regardless. Returns `None` for a URL that doesn't even parse, so the
/// row falls through to a real capture attempt (which will fail with a
/// clear, per-row error instead of being silently skipped here).
fn precheck_link(raw_url: &str) -> Option<String> {
    let parsed = Url::parse(raw_url.trim()).ok()?;
    Some(strip_tracking_params(canonicalize(&parsed).as_url()).to_string())
}

async fn process_row(
    http_client: reqwest::Client,
    data_dir: std::path::PathBuf,
    pool: db::DbPool,
    row: RaindropRow,
) -> RowOutcome {
    let url = row.url.trim().to_string();
    let display_title = if row.title.trim().is_empty() {
        url.clone()
    } else {
        row.title.clone()
    };

    let id = uuid::Uuid::new_v4().to_string();
    let output = match capture::capture_local(&http_client, &data_dir, &id, &url).await {
        Ok(output) => output,
        Err(err) => {
            return RowOutcome::Failed(ImportFailure {
                url,
                title: display_title,
                error: err.to_string(),
            });
        }
    };

    let tags = parse_tags(&row.tags);
    let favorited = row.favorite.trim().eq_ignore_ascii_case("true");
    let saved_at = row.created.clone();

    let conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            return RowOutcome::Failed(ImportFailure {
                url,
                title: display_title,
                error: err.to_string(),
            });
        }
    };
    match queries::insert_imported_article(
        &conn,
        &id,
        SOURCE_NAME,
        &output,
        &tags,
        &saved_at,
        favorited,
    ) {
        Ok(true) => RowOutcome::Imported,
        Ok(false) => RowOutcome::SkippedDuplicate,
        Err(err) => RowOutcome::Failed(ImportFailure {
            url,
            title: display_title,
            error: err.to_string(),
        }),
    }
}

fn progress_payload(summary: &ImportSummary, processed: u32) -> ImportProgress {
    ImportProgress {
        processed,
        total: summary.total,
        imported: summary.imported,
        skipped_duplicate: summary.skipped_duplicate,
        failed: summary.failed.len() as u32,
    }
}

/// Cheaply checks that `csv_bytes` parses as a Raindrop export at all,
/// without keeping the parsed rows around — used by
/// `commands::import::import_raindrop_csv` to reject an obviously-wrong
/// file synchronously, before it reports itself as "running" via
/// [`ImportEvent::Started`].
pub fn validate_csv(csv_bytes: &[u8]) -> Result<(), csv::Error> {
    let mut reader = csv::Reader::from_reader(csv_bytes);
    for row in reader.deserialize::<RaindropRow>() {
        row?;
    }
    Ok(())
}

/// Parses `csv_bytes` (a Raindrop.io bookmark export) and imports every
/// row it can, invoking `on_event` as it goes (see [`ImportEvent`]).
///
/// `cancel` is polled between dispatching rows to the worker pool, not
/// mid-fetch: once set, no *new* row is dispatched, but up to
/// `CONCURRENCY` already in-flight captures are allowed to finish so
/// their network fetch isn't wasted. The returned [`ImportSummary`]
/// reflects whatever completed either way.
///
/// Returns `Err` only for a CSV the `csv` crate can't parse at all (wrong
/// file entirely) — a single malformed *row* within an otherwise valid
/// file isn't distinguished from any other per-row failure by this
/// function; `csv::Reader::deserialize` surfaces both as `Err` items in
/// the same iterator, and both are treated as fatal here rather than
/// silently dropping rows a user might expect to see reported.
pub async fn run_import(
    state: &AppState,
    csv_bytes: Vec<u8>,
    cancel: Arc<AtomicBool>,
    mut on_event: impl FnMut(ImportEvent),
) -> Result<ImportSummary, csv::Error> {
    let mut reader = csv::Reader::from_reader(csv_bytes.as_slice());
    let rows: Vec<RaindropRow> = reader
        .deserialize::<RaindropRow>()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|row| !row.url.trim().is_empty())
        .collect();

    let total = rows.len() as u32;
    on_event(ImportEvent::Started { total });

    let mut summary = ImportSummary {
        total,
        ..Default::default()
    };

    let mut rows_iter = rows.into_iter().map(|row| {
        let link_hint = precheck_link(&row.url);
        (row, link_hint)
    });
    let mut join_set = tokio::task::JoinSet::new();
    let mut processed = 0u32;
    let mut since_last_library_refresh = 0u32;

    loop {
        // Top up the worker pool up to `CONCURRENCY`, skipping rows whose
        // cleaned link is already stored (no network request paid for
        // those) and stopping entirely once cancellation is requested.
        while join_set.len() < CONCURRENCY {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let Some((row, link_hint)) = rows_iter.next() else {
                break;
            };

            let already_exists = match &link_hint {
                Some(link) => state
                    .pool
                    .get()
                    .ok()
                    .and_then(|conn| queries::article_link_exists(&conn, link).ok())
                    .unwrap_or(false),
                None => false,
            };
            if already_exists {
                processed += 1;
                summary.skipped_duplicate += 1;
                on_event(ImportEvent::Progress(progress_payload(&summary, processed)));
                continue;
            }

            let http_client = state.http_client.clone();
            let data_dir = state.data_dir.clone();
            let pool = state.pool.clone();
            join_set.spawn(process_row(http_client, data_dir, pool, row));
        }

        let Some(joined) = join_set.join_next().await else {
            // Nothing in flight and nothing left to dispatch (or
            // cancelled with the pool already drained) — done.
            break;
        };

        processed += 1;
        match joined {
            Ok(RowOutcome::Imported) => {
                summary.imported += 1;
                since_last_library_refresh += 1;
            }
            Ok(RowOutcome::SkippedDuplicate) => summary.skipped_duplicate += 1,
            Ok(RowOutcome::Failed(failure)) => summary.failed.push(failure),
            Err(join_err) => summary.failed.push(ImportFailure {
                url: String::new(),
                title: String::new(),
                error: join_err.to_string(),
            }),
        }
        on_event(ImportEvent::Progress(progress_payload(&summary, processed)));

        if since_last_library_refresh >= LIBRARY_REFRESH_BATCH {
            since_last_library_refresh = 0;
            on_event(ImportEvent::LibraryChanged);
        }
    }

    summary.cancelled = cancel.load(Ordering::Relaxed);
    if summary.imported > 0 {
        on_event(ImportEvent::LibraryChanged);
    }
    on_event(ImportEvent::Finished(ImportFinished {
        total: summary.total,
        imported: summary.imported,
        skipped_duplicate: summary.skipped_duplicate,
        failed: summary.failed.clone(),
        cancelled: summary.cancelled,
    }));

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use tokio::sync::Mutex;

    use super::*;
    use crate::test_support;

    fn csv_bytes(rows: &[(&str, &str, &str, &str, &str)]) -> Vec<u8> {
        let mut out = String::from(
            "id,title,note,excerpt,url,folder,tags,created,cover,highlights,favorite\n",
        );
        for (title, url, tags, created, favorite) in rows {
            out.push_str(&format!(
                "1,{title},,,{url},Unsorted,\"{tags}\",{created},,,{favorite}\n"
            ));
        }
        out.into_bytes()
    }

    fn build_state(data_dir: &std::path::Path) -> AppState {
        let db_path = data_dir.join("legere.db");
        let pool = db::build_pool(&db_path).expect("build pool");
        {
            let mut conn = pool.get().expect("get conn");
            db::schema::migrate(&mut conn).expect("migrate");
        }
        AppState {
            pool,
            http_client: test_support::plain_client(),
            data_dir: data_dir.to_path_buf(),
            autosync_handle: Mutex::new(None),
            last_foreground_sync: std::sync::Mutex::new(None),
            import_cancel: Mutex::new(None),
            pending_update: Default::default(),
            pending_linux_update: Default::default(),
        }
    }

    #[tokio::test]
    async fn imports_a_row_with_its_tags_and_favorite_flag() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let base_url = test_support::spawn().await;
        let state = build_state(data_dir.path());

        let csv = csv_bytes(&[(
            "First",
            &format!("{base_url}/article.html?a=1"),
            "tag-one, tag-two",
            "2024-01-01T00:00:00Z",
            "true",
        )]);

        let summary = run_import(&state, csv, Arc::new(AtomicBool::new(false)), |_| {})
            .await
            .expect("valid csv should parse");

        assert_eq!(summary.total, 1);
        assert_eq!(summary.imported, 1);
        assert_eq!(summary.skipped_duplicate, 0);
        assert!(summary.failed.is_empty());

        let conn = state.pool.get().unwrap();
        let articles = queries::list_articles(&conn).unwrap();
        assert_eq!(articles.len(), 1);
        assert_eq!(articles[0].source_name, SOURCE_NAME);
        assert_eq!(articles[0].source_type, "direct");
        assert!(articles[0].favorited);
        assert_eq!(
            articles[0].tags,
            vec!["tag-one".to_string(), "tag-two".to_string()]
        );
    }

    /// Two rows racing on the *same* link within a single `run_import`
    /// call is exercised by `precheck_link`'s own doc comment (the
    /// pre-check is best-effort, not authoritative); which one of the two
    /// concurrent captures actually wins isn't deterministic, so this
    /// test instead proves the case the pre-check *is* meant to catch:
    /// a link already sitting in the database from a wholly separate,
    /// already-finished import run.
    #[tokio::test]
    async fn skips_a_link_already_imported_by_an_earlier_run() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let base_url = test_support::spawn().await;
        let state = build_state(data_dir.path());
        let url = format!("{base_url}/article.html?dedup=1");

        let first = run_import(
            &state,
            csv_bytes(&[("First", &url, "", "2024-01-01T00:00:00Z", "false")]),
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
        .await
        .expect("valid csv should parse");
        assert_eq!(first.imported, 1);

        let second = run_import(
            &state,
            csv_bytes(&[("Second", &url, "", "2024-01-02T00:00:00Z", "false")]),
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
        .await
        .expect("valid csv should parse");
        assert_eq!(second.imported, 0);
        assert_eq!(second.skipped_duplicate, 1);

        let conn = state.pool.get().unwrap();
        assert_eq!(queries::list_articles(&conn).unwrap().len(), 1);
    }

    #[tokio::test]
    async fn records_a_dead_link_as_a_failure_without_aborting_the_batch() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let base_url = test_support::spawn().await;
        let state = build_state(data_dir.path());

        let csv = csv_bytes(&[
            (
                "Missing",
                &format!("{base_url}/does-not-exist.html"),
                "",
                "2024-01-01T00:00:00Z",
                "false",
            ),
            (
                "Real",
                &format!("{base_url}/article.html?ok=1"),
                "",
                "2024-01-01T00:00:00Z",
                "false",
            ),
        ]);

        let summary = run_import(&state, csv, Arc::new(AtomicBool::new(false)), |_| {})
            .await
            .expect("valid csv should parse");

        assert_eq!(summary.total, 2);
        assert_eq!(summary.imported, 1);
        assert_eq!(summary.failed.len(), 1);
        assert!(summary.failed[0].url.contains("does-not-exist.html"));
    }

    #[tokio::test]
    async fn empty_url_rows_are_excluded_from_the_total() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let base_url = test_support::spawn().await;
        let state = build_state(data_dir.path());

        let csv = csv_bytes(&[
            ("No URL", "", "", "2024-01-01T00:00:00Z", "false"),
            (
                "Real",
                &format!("{base_url}/article.html?empty-url-test=1"),
                "",
                "2024-01-01T00:00:00Z",
                "false",
            ),
        ]);

        let summary = run_import(&state, csv, Arc::new(AtomicBool::new(false)), |_| {})
            .await
            .expect("valid csv should parse");

        assert_eq!(
            summary.total, 1,
            "the blank-url row must not count toward the total"
        );
        assert_eq!(summary.imported, 1);
    }

    #[tokio::test]
    async fn setting_cancel_before_starting_dispatches_nothing() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let base_url = test_support::spawn().await;
        let state = build_state(data_dir.path());

        let csv = csv_bytes(&[(
            "Real",
            &format!("{base_url}/article.html?cancel-test=1"),
            "",
            "2024-01-01T00:00:00Z",
            "false",
        )]);

        let summary = run_import(&state, csv, Arc::new(AtomicBool::new(true)), |_| {})
            .await
            .expect("valid csv should parse");

        assert_eq!(summary.total, 1);
        assert_eq!(summary.imported, 0);
        assert!(summary.cancelled);
    }
}
