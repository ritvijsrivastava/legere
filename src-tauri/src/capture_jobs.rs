//! In-memory tracking for "Add a source" requests that now run in the
//! background (`commands::sources::add_source_background`) instead of
//! blocking the dialog until the fetch/extract/localize pipeline
//! finishes — a slow site or an image-heavy article could otherwise leave
//! the "Add a source" dialog stuck for tens of seconds with nothing to do
//! but wait.
//!
//! Deliberately in-memory only (`AppState::capture_jobs`), not persisted
//! to SQLite: a job is transient bookkeeping for "is this add still in
//! flight, and if it failed, what do I retry" — the moment it succeeds,
//! whatever it produced (an article or a source row) is real, durable
//! data covered by the usual `articles:changed`/`source:changed` events,
//! and this list has nothing further to say about it. Losing an
//! in-progress/failed job list across an app restart is an accepted
//! tradeoff of that scope, matching `AppState::import_cancel`'s own
//! session-lifetime-only guard.
//!
//! A job is removed from the list the instant it succeeds; only
//! `Running` and `Failed` jobs are ever returned by `list` — there is
//! deliberately no `Done` state for a caller to have to filter out.

use std::sync::Mutex;

use serde::Serialize;
use tauri::async_runtime::JoinHandle;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CaptureJobStatus {
    Running,
    Failed { message: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct CaptureJob {
    pub id: String,
    pub url: String,
    pub status: CaptureJobStatus,
}

struct JobEntry {
    job: CaptureJob,
    /// The background task actually running this job's capture, so
    /// `cancel` can abort it outright rather than needing a cooperative
    /// flag threaded through `capture::capture_local`. `None` once the
    /// job has settled into `Failed` — there's nothing left to abort.
    /// Aborting mid-pipeline is always safe: nothing is written to disk
    /// or the database until a capture's very last step (see
    /// `capture::archive`), so cutting it off at any earlier point
    /// leaves no partial state behind.
    handle: Option<JoinHandle<()>>,
}

/// A plain (non-async) `std::sync::Mutex` rather than `tokio::sync::Mutex`
/// — every method here only ever does quick in-memory `Vec` bookkeeping
/// and returns before any `.await`, so there's nothing to gain from an
/// async-aware lock, and a sync one lets `spawn_capture_job` attach a
/// freshly-spawned task's handle without a second `spawn` just to do that.
#[derive(Default)]
pub struct CaptureJobs(Mutex<Vec<JobEntry>>);

impl CaptureJobs {
    /// Newest first, matching how the frontend's activity dock wants to
    /// show whatever was most recently kicked off at the top.
    pub fn list(&self) -> Vec<CaptureJob> {
        let mut jobs: Vec<CaptureJob> = self
            .0
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.job.clone())
            .collect();
        jobs.reverse();
        jobs
    }

    pub fn enqueue(&self, url: String) -> CaptureJob {
        let job = CaptureJob {
            id: uuid::Uuid::new_v4().to_string(),
            url,
            status: CaptureJobStatus::Running,
        };
        self.0.lock().unwrap().push(JobEntry {
            job: job.clone(),
            handle: None,
        });
        job
    }

    /// Attaches the background task's handle once it exists (it can only
    /// be spawned after `enqueue` already returned the job to the
    /// caller) — a no-op if `id` has already settled (succeeded, failed,
    /// or was dismissed) by the time this runs, same as every other
    /// lookup-by-id method here.
    pub fn attach_handle(&self, id: &str, handle: JoinHandle<()>) {
        let mut jobs = self.0.lock().unwrap();
        if let Some(entry) = jobs.iter_mut().find(|e| e.job.id == id) {
            entry.handle = Some(handle);
        }
    }

    /// Puts an existing (non-running) job back to `Running` for a retry
    /// attempt, returning the URL to re-run it with. `None` if `id` isn't
    /// a known job, or is already running — the caller shouldn't have
    /// offered a retry button for either case, but this stays defensive
    /// rather than dispatching a second concurrent attempt at the same
    /// job id.
    pub fn retry(&self, id: &str) -> Option<String> {
        let mut jobs = self.0.lock().unwrap();
        let entry = jobs.iter_mut().find(|e| e.job.id == id)?;
        if entry.job.status == CaptureJobStatus::Running {
            return None;
        }
        entry.job.status = CaptureJobStatus::Running;
        entry.handle = None;
        Some(entry.job.url.clone())
    }

    /// A successful capture has nothing left to show here — the article
    /// or source it produced is now real library data, reported through
    /// the usual `articles:changed`/`source:changed` events instead.
    pub fn succeed(&self, id: &str) {
        self.0.lock().unwrap().retain(|e| e.job.id != id);
    }

    pub fn fail(&self, id: &str, message: String) {
        let mut jobs = self.0.lock().unwrap();
        if let Some(entry) = jobs.iter_mut().find(|e| e.job.id == id) {
            entry.job.status = CaptureJobStatus::Failed { message };
            entry.handle = None;
        }
    }

    /// Drops an acknowledged failure the user chose not to retry. A
    /// no-op if `id` is currently running (or already gone) — the
    /// frontend shouldn't offer a dismiss button for a running job (it
    /// offers `cancel` instead), but this stays defensive rather than
    /// ever discarding one still in flight.
    pub fn dismiss(&self, id: &str) {
        let mut jobs = self.0.lock().unwrap();
        jobs.retain(|e| e.job.id != id || e.job.status == CaptureJobStatus::Running);
    }

    /// Aborts a still-running job's background task outright and drops
    /// it from the list. Returns `false` if `id` isn't a known, running
    /// job (already settled, or unknown) — the caller shouldn't have
    /// offered a cancel button for either case, but this stays
    /// defensive.
    pub fn cancel(&self, id: &str) -> bool {
        let mut jobs = self.0.lock().unwrap();
        let Some(pos) = jobs
            .iter()
            .position(|e| e.job.id == id && e.job.status == CaptureJobStatus::Running)
        else {
            return false;
        };
        let entry = jobs.remove(pos);
        if let Some(handle) = entry.handle {
            handle.abort();
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueue_lists_newest_first() {
        let jobs = CaptureJobs::default();
        let first = jobs.enqueue("https://a.example".to_string());
        let second = jobs.enqueue("https://b.example".to_string());
        let listed = jobs.list();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, second.id);
        assert_eq!(listed[1].id, first.id);
    }

    #[test]
    fn succeed_removes_the_job() {
        let jobs = CaptureJobs::default();
        let job = jobs.enqueue("https://a.example".to_string());
        jobs.succeed(&job.id);
        assert!(jobs.list().is_empty());
    }

    #[test]
    fn fail_then_retry_round_trip() {
        let jobs = CaptureJobs::default();
        let job = jobs.enqueue("https://a.example".to_string());
        jobs.fail(&job.id, "boom".to_string());
        assert_eq!(
            jobs.list()[0].status,
            CaptureJobStatus::Failed {
                message: "boom".to_string()
            }
        );

        let url = jobs.retry(&job.id).expect("known failed job");
        assert_eq!(url, "https://a.example");
        assert_eq!(jobs.list()[0].status, CaptureJobStatus::Running);
    }

    #[test]
    fn retry_refuses_an_already_running_job() {
        let jobs = CaptureJobs::default();
        let job = jobs.enqueue("https://a.example".to_string());
        assert_eq!(jobs.retry(&job.id), None);
    }

    #[test]
    fn dismiss_drops_a_failed_job_but_not_a_running_one() {
        let jobs = CaptureJobs::default();
        let failed = jobs.enqueue("https://a.example".to_string());
        jobs.fail(&failed.id, "boom".to_string());
        let running = jobs.enqueue("https://b.example".to_string());

        jobs.dismiss(&failed.id);
        jobs.dismiss(&running.id);

        let listed = jobs.list();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, running.id);
    }

    #[tokio::test]
    async fn cancel_aborts_the_running_task_and_removes_the_job() {
        let jobs = CaptureJobs::default();
        let job = jobs.enqueue("https://a.example".to_string());

        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let handle = tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let _ = tx.send(());
        });
        jobs.attach_handle(&job.id, handle);

        assert!(jobs.cancel(&job.id));
        assert!(jobs.list().is_empty());

        // The task never got to send on `tx` — proof it was actually
        // aborted, not just dropped from the list while still running.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        assert!(matches!(
            rx.await,
            Err(tokio::sync::oneshot::error::RecvError { .. })
        ));
    }

    #[test]
    fn cancel_refuses_an_unknown_or_already_failed_job() {
        let jobs = CaptureJobs::default();
        assert!(!jobs.cancel("missing"));

        let job = jobs.enqueue("https://a.example".to_string());
        jobs.fail(&job.id, "boom".to_string());
        assert!(!jobs.cancel(&job.id));
    }
}
