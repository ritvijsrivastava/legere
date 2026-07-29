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
