//! Backend -> frontend event emission (`AppHandle::emit`). The frontend
//! registers listeners in `lib/events.ts` and responds to all four of
//! these by refetching the relevant store — coarse-grained refetch rather
//! than payload-carried deltas, since the dataset is small and this is
//! what actually fixes the original bug: nothing told the library to
//! refresh after a background autosync tick, so newly captured articles
//! were invisible until the app restarted.

use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
pub struct SyncError {
    pub source_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncFinished {
    pub new_article_count: u32,
    pub errors: Vec<SyncError>,
}

/// `scope` is `"all"` for the autosync/manual-sync-all path, or a source
/// id for a single-source sync.
pub fn emit_sync_started(app: &AppHandle, scope: &str) {
    let _ = app.emit("sync:started", scope);
}

pub fn emit_sync_finished(app: &AppHandle, payload: &SyncFinished) {
    let _ = app.emit("sync:finished", payload);
}

pub fn emit_articles_changed(app: &AppHandle) {
    let _ = app.emit("articles:changed", ());
}

pub fn emit_source_changed(app: &AppHandle) {
    let _ = app.emit("source:changed", ());
}

/// A category was created, renamed, or deleted, or an article's category
/// assignment changed — a cue for the sidebar's category list
/// (`commands::categories::get_categories`) to refetch.
pub fn emit_category_changed(app: &AppHandle) {
    let _ = app.emit("category:changed", ());
}

/// One row `raindrop_import::run_import` couldn't capture — a dead link,
/// timeout, or DB error. Collected rather than aborting the whole import,
/// since a multi-thousand-row, years-old export is expected to contain
/// plenty of these.
#[derive(Debug, Clone, Serialize)]
pub struct ImportFailure {
    pub url: String,
    pub title: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportProgress {
    pub processed: u32,
    pub total: u32,
    pub imported: u32,
    pub skipped_duplicate: u32,
    pub failed: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportFinished {
    pub total: u32,
    pub imported: u32,
    pub skipped_duplicate: u32,
    pub failed: Vec<ImportFailure>,
    /// `true` if `cancel_raindrop_import` stopped this run early — the
    /// counts above still reflect whatever completed before that point.
    pub cancelled: bool,
}

pub fn emit_import_started(app: &AppHandle, total: u32) {
    let _ = app.emit("import:started", total);
}

pub fn emit_import_progress(app: &AppHandle, payload: &ImportProgress) {
    let _ = app.emit("import:progress", payload);
}

pub fn emit_import_finished(app: &AppHandle, payload: &ImportFinished) {
    let _ = app.emit("import:finished", payload);
}

/// Same three lifecycle events as `import:*` above, emitted instead by
/// Settings' "Import articles" (`exports::articles::run_import`, Legere's
/// own CSV shape) so the two importers' progress can never be confused on
/// the frontend even though they share the exact same payload shapes
/// (`ImportProgress`/`ImportFinished`/`ImportFailure`) — both are "parse
/// rows, capture each through the same pipeline, report progress" at
/// heart, just fed from different CSV column sets.
pub fn emit_article_import_started(app: &AppHandle, total: u32) {
    let _ = app.emit("article_import:started", total);
}

pub fn emit_article_import_progress(app: &AppHandle, payload: &ImportProgress) {
    let _ = app.emit("article_import:progress", payload);
}

pub fn emit_article_import_finished(app: &AppHandle, payload: &ImportFinished) {
    let _ = app.emit("article_import:finished", payload);
}

/// A background "add a source" job (`capture_jobs::CaptureJobs`) was
/// enqueued, retried, dismissed, succeeded, or failed — a cue for the
/// frontend's activity panel to refetch `list_capture_jobs`. Coarse-
/// grained like every other `*_changed` event here: the job list is tiny,
/// so a full refetch is simpler than trying to keep a payload in sync
/// with exactly what changed.
pub fn emit_capture_changed(app: &AppHandle) {
    let _ = app.emit("capture:changed", ());
}

#[derive(Debug, Clone, Serialize)]
pub struct CaptureSucceeded {
    /// `"rss"` or `"direct"` — lets the frontend word the confirmation
    /// toast appropriately ("Following" vs. "Added").
    pub kind: String,
    pub title: String,
}

/// A background capture landed successfully. Separate from
/// `emit_capture_changed` (which only says "go refetch the job list")
/// because this one carries just enough to word a one-off success toast
/// — the dialog closed the instant the job was queued, so this is the
/// only feedback the user gets that it actually worked.
pub fn emit_capture_succeeded(app: &AppHandle, payload: &CaptureSucceeded) {
    let _ = app.emit("capture:succeeded", payload);
}
