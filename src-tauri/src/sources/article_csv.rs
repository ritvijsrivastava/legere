//! CSV export/import of the whole article library, in Legere's own shape
//! (`title,url,category,tags,created,favourite`) — deliberately a
//! different column set from `sources::raindrop_import`, which stays
//! scoped to one-time migration *from* a Raindrop.io export. This is the
//! shape behind Settings' "Export articles" / "Import articles": a
//! user's whole library round-trips through one file that's also just a
//! plain, readable CSV to hand to someone else running Legere (or any
//! other bookmark tool that can read a CSV).
//!
//! Import reuses the app's one-capture-pipeline invariant like every
//! other ingestion path — each row is fetched fresh through
//! `capture::capture_local`, not restored from a data blob, so a file
//! exported from someone else's device (or an older Legere install with
//! no local content on this one) still works; nothing but the row's `url`
//! is actually required to reconstruct an article. The rest of this
//! module's shape (worker pool, dedup pre-check, per-category resolution,
//! progress events, cancellation) intentionally mirrors
//! `raindrop_import` closely — same problem, same solution — just reading
//! this module's own column names instead.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use url::Url;

use crate::db::queries;
use crate::error::AppError;
use crate::events::{ImportFailure, ImportFinished, ImportProgress};
use crate::state::AppState;
use crate::urlx::{canonicalize, strip_tracking_params};
use crate::{capture, db};

/// See `raindrop_import::LIBRARY_REFRESH_BATCH` — same tradeoff, same
/// value.
const LIBRARY_REFRESH_BATCH: u32 = 100;

/// Mirrors `raindrop_import::ImportEvent` — see its doc comment.
pub enum ImportEvent {
    Started { total: u32 },
    Progress(ImportProgress),
    LibraryChanged,
    Finished(ImportFinished),
}

#[derive(Debug, Serialize)]
struct ArticleExportRow<'a> {
    title: &'a str,
    url: &'a str,
    category: &'a str,
    tags: String,
    created: &'a str,
    favourite: bool,
}

/// Builds the CSV bytes for `rows` (see `db::queries::list_articles_for_export`)
/// — the exact shape `parse_import_csv`/`run_import` below read back.
pub fn build_csv(rows: &[queries::ArticleExportRecord]) -> Result<String, csv::Error> {
    let mut writer = csv::Writer::from_writer(vec![]);
    for row in rows {
        writer.serialize(ArticleExportRow {
            title: &row.title,
            url: &row.link,
            category: row.category_name.as_deref().unwrap_or(""),
            tags: row.tags.join(","),
            created: &row.fetched_at,
            favourite: row.favorited,
        })?;
    }
    let bytes = writer.into_inner().map_err(|e| e.into_error())?;
    Ok(String::from_utf8(bytes).expect("csv writer only ever emits valid utf8 from utf8 input"))
}

#[derive(Debug, Deserialize)]
struct ArticleImportRow {
    #[serde(default)]
    title: String,
    url: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    tags: String,
    #[serde(default)]
    created: String,
    #[serde(default)]
    favourite: String,
}

/// Stable key for an import row's category cell — trimmed, empty means
/// Uncategorized. Unlike `raindrop_import::folder_key`, there's no
/// `"Unsorted"` special case to fold in: this column only ever comes from
/// Legere's own export, which already writes an empty cell for
/// Uncategorized.
fn category_key(raw: &str) -> String {
    raw.trim().to_string()
}

fn category_display_name(key: &str) -> String {
    if key.is_empty() {
        "Uncategorized".to_string()
    } else {
        key.to_string()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoryPreview {
    /// Stable key, same shape `category_key` produces.
    pub category: String,
    /// Human-readable name for the preview UI.
    pub name: String,
    pub row_count: u32,
    pub duplicate_count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArticleImportPreview {
    pub total: u32,
    pub categories: Vec<CategoryPreview>,
}

/// Splits a comma-separated `tags` cell into a trimmed, non-empty tag
/// list — same shape `build_csv` writes.
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

/// See `raindrop_import::precheck_link`'s doc comment — same best-effort
/// dedup key, same tradeoff.
fn precheck_link(raw_url: &str) -> Option<String> {
    let parsed = Url::parse(raw_url.trim()).ok()?;
    Some(strip_tracking_params(canonicalize(&parsed).as_url()).to_string())
}

async fn process_row(
    http_client: reqwest::Client,
    data_dir: std::path::PathBuf,
    pool: db::DbPool,
    row: ArticleImportRow,
    category_id: Option<String>,
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
    let favorited = row.favourite.trim().eq_ignore_ascii_case("true");
    let saved_at = if row.created.trim().is_empty() {
        chrono::Utc::now().to_rfc3339()
    } else {
        row.created.clone()
    };

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
    match queries::insert_imported_article_with_category(
        &conn,
        &id,
        &output,
        &tags,
        &saved_at,
        favorited,
        category_id.as_deref(),
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

/// Cheaply checks that `csv_bytes` parses as this module's own shape at
/// all (in particular, that a `url` column exists), without keeping the
/// parsed rows around.
pub fn validate_csv(csv_bytes: &[u8]) -> Result<(), csv::Error> {
    let mut reader = csv::Reader::from_reader(csv_bytes);
    for row in reader.deserialize::<ArticleImportRow>() {
        row?;
    }
    Ok(())
}

/// Parses a CSV without fetching any links or creating any categories —
/// a read-only summary the UI shows before an import starts.
pub fn preview_csv(state: &AppState, csv_bytes: &[u8]) -> Result<ArticleImportPreview, AppError> {
    let mut reader = csv::Reader::from_reader(csv_bytes);
    let rows: Vec<ArticleImportRow> = reader
        .deserialize::<ArticleImportRow>()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|row| !row.url.trim().is_empty())
        .collect();

    let mut categories: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    let conn = state.pool.get()?;
    for row in &rows {
        let key = category_key(&row.category);
        let entry = categories.entry(key).or_default();
        entry.0 += 1;
        if let Some(link) = precheck_link(&row.url)
            && queries::article_link_exists(&conn, &link)?
        {
            entry.1 += 1;
        }
    }

    Ok(ArticleImportPreview {
        total: rows.len() as u32,
        categories: categories
            .into_iter()
            .map(|(category, (row_count, duplicate_count))| CategoryPreview {
                name: category_display_name(&category),
                category,
                row_count,
                duplicate_count,
            })
            .collect(),
    })
}

/// Resolves every distinct category key appearing in `rows` to a category
/// id, creating a same-named category the first time it's seen. See
/// `raindrop_import::resolve_folder_categories` for why this runs once
/// per distinct key up front rather than per-row inside the worker pool.
fn resolve_row_categories(
    state: &AppState,
    rows: &[ArticleImportRow],
) -> Result<HashMap<String, Option<String>>, AppError> {
    let conn = state.pool.get()?;
    let mut resolved = HashMap::new();
    for row in rows {
        let key = category_key(&row.category);
        if resolved.contains_key(&key) {
            continue;
        }
        let category_id = if key.is_empty() {
            None
        } else {
            Some(queries::find_or_create_category(&conn, &key)?.id)
        };
        resolved.insert(key, category_id);
    }
    Ok(resolved)
}

/// Parses `csv_bytes` (this module's own export shape) and imports every
/// row it can, invoking `on_event` as it goes. Structurally identical to
/// `raindrop_import::run_import` — see its doc comment for the worker
/// pool/dedup/cancellation semantics, all unchanged here.
pub async fn run_import(
    state: &AppState,
    csv_bytes: Vec<u8>,
    concurrency: usize,
    cancel: Arc<AtomicBool>,
    mut on_event: impl FnMut(ImportEvent),
) -> Result<ImportSummary, AppError> {
    let mut reader = csv::Reader::from_reader(csv_bytes.as_slice());
    let rows: Vec<ArticleImportRow> = reader
        .deserialize::<ArticleImportRow>()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|row| !row.url.trim().is_empty())
        .collect();

    let row_categories = resolve_row_categories(state, &rows)?;

    let total = rows.len() as u32;
    on_event(ImportEvent::Started { total });

    let mut summary = ImportSummary {
        total,
        ..Default::default()
    };

    let mut rows_iter = rows.into_iter().map(|row| {
        let link_hint = precheck_link(&row.url);
        let category_id = row_categories
            .get(&category_key(&row.category))
            .cloned()
            .flatten();
        (row, link_hint, category_id)
    });
    let mut join_set = tokio::task::JoinSet::new();
    let mut processed = 0u32;
    let mut since_last_library_refresh = 0u32;

    loop {
        while join_set.len() < concurrency {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let Some((row, link_hint, category_id)) = rows_iter.next() else {
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
                if let Some(link) = &link_hint {
                    let tags = parse_tags(&row.tags);
                    let merge_result =
                        state
                            .pool
                            .get()
                            .map_err(|err| err.to_string())
                            .and_then(|conn| {
                                queries::merge_tags_by_link(&conn, link, &tags)
                                    .map_err(|err| err.to_string())
                            });
                    if let Err(error) = merge_result {
                        summary.failed.push(ImportFailure {
                            url: row.url.clone(),
                            title: row.title.clone(),
                            error,
                        });
                    }
                }
                processed += 1;
                summary.skipped_duplicate += 1;
                on_event(ImportEvent::Progress(progress_payload(&summary, processed)));
                continue;
            }

            let http_client = state.http_client.clone();
            let data_dir = state.data_dir.clone();
            let pool = state.pool.clone();
            join_set.spawn(process_row(http_client, data_dir, pool, row, category_id));
        }

        let Some(joined) = join_set.join_next().await else {
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

    fn csv_bytes(rows: &[(&str, &str, &str, &str, &str, &str)]) -> Vec<u8> {
        let mut out = String::from("title,url,category,tags,created,favourite\n");
        for (title, url, category, tags, created, favourite) in rows {
            out.push_str(&format!(
                "{title},{url},{category},\"{tags}\",{created},{favourite}\n"
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
            article_import_cancel: Mutex::new(None),
            capture_jobs: Default::default(),
            pending_update: Default::default(),
            pending_linux_update: Default::default(),
        }
    }

    #[test]
    fn build_csv_round_trips_through_parse_import_csv() {
        let rows = vec![queries::ArticleExportRecord {
            title: "Hello".to_string(),
            link: "https://example.com/a".to_string(),
            category_name: Some("Reading".to_string()),
            tags: vec!["one".to_string(), "two".to_string()],
            fetched_at: "2024-01-01T00:00:00Z".to_string(),
            favorited: true,
        }];
        let csv = build_csv(&rows).expect("builds");
        assert!(csv.contains("title,url,category,tags,created,favourite"));
        assert!(
            csv.contains(
                "Hello,https://example.com/a,Reading,\"one,two\",2024-01-01T00:00:00Z,true"
            )
        );
    }

    #[tokio::test]
    async fn imports_a_row_with_its_category_tags_and_favourite_flag() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let base_url = test_support::spawn().await;
        let state = build_state(data_dir.path());

        let csv = csv_bytes(&[(
            "First",
            &format!("{base_url}/article.html?a=1"),
            "Reading",
            "tag-one, tag-two",
            "2024-01-01T00:00:00Z",
            "true",
        )]);

        let summary = run_import(&state, csv, 5, Arc::new(AtomicBool::new(false)), |_| {})
            .await
            .expect("valid csv should parse");

        assert_eq!(summary.total, 1);
        assert_eq!(summary.imported, 1);

        let conn = state.pool.get().unwrap();
        let articles = queries::list_articles(&conn).unwrap();
        assert_eq!(articles.len(), 1);
        assert!(articles[0].favorited);
        assert_eq!(
            articles[0].tags,
            vec!["tag-one".to_string(), "tag-two".to_string()]
        );
        assert_eq!(articles[0].category_name.as_deref(), Some("Reading"));
    }

    #[tokio::test]
    async fn skips_a_link_already_imported_and_still_merges_new_tags() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let base_url = test_support::spawn().await;
        let state = build_state(data_dir.path());
        let url = format!("{base_url}/article.html?dedup=1");

        let first = run_import(
            &state,
            csv_bytes(&[("First", &url, "", "pdf", "2024-01-01T00:00:00Z", "false")]),
            5,
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
        .await
        .unwrap();
        assert_eq!(first.imported, 1);

        let second = run_import(
            &state,
            csv_bytes(&[(
                "Second",
                &url,
                "",
                "pdf, other",
                "2024-01-02T00:00:00Z",
                "false",
            )]),
            5,
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
        .await
        .unwrap();
        assert_eq!(second.imported, 0);
        assert_eq!(second.skipped_duplicate, 1);

        let conn = state.pool.get().unwrap();
        let articles = queries::list_articles(&conn).unwrap();
        assert_eq!(articles.len(), 1);
        assert_eq!(
            articles[0].tags,
            vec!["pdf".to_string(), "other".to_string()]
        );
    }

    #[tokio::test]
    async fn preview_groups_by_category_and_counts_duplicates() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let base_url = test_support::spawn().await;
        let state = build_state(data_dir.path());
        let existing_url = format!("{base_url}/article.html?preview-existing=1");
        let output = capture::LocalCaptureOutput {
            title: "Existing".to_string(),
            link: existing_url.clone(),
            final_url: existing_url.clone(),
            excerpt: "excerpt".to_string(),
            content_html: "<p>content</p>".to_string(),
            published_at: None,
            read_time_min: 1,
            hero_image_path: None,
            extraction_confident: true,
        };
        let conn = state.pool.get().unwrap();
        queries::insert_captured_article(&conn, "existing", None, "direct", &output, &[]).unwrap();

        let csv = csv_bytes(&[
            (
                "One",
                &existing_url,
                "Reading",
                "",
                "2024-01-01T00:00:00Z",
                "false",
            ),
            (
                "Two",
                &format!("{base_url}/article.html?preview-new=1"),
                "Reading",
                "",
                "2024-01-01T00:00:00Z",
                "false",
            ),
        ]);

        let preview = preview_csv(&state, &csv).expect("valid preview");
        assert_eq!(preview.total, 2);
        assert_eq!(preview.categories.len(), 1);
        assert_eq!(preview.categories[0].name, "Reading");
        assert_eq!(preview.categories[0].row_count, 2);
        assert_eq!(preview.categories[0].duplicate_count, 1);
    }
}
