import type { ImportFailure, ImportFinished, ImportProgress } from '$lib/types';

/** Tracks a Raindrop CSV import's lifecycle, driven entirely by the
 *  backend's `import:*` events (see `lib/events.ts`) rather than the
 *  triggering command's return value \u2014 the import runs in the background
 *  and can outlive the dialog that started it, so this is a module-level
 *  store (not local component state) and reflects whatever's currently
 *  running (or last finished) regardless of which component reads it. */
class ImportStore {
	running = $state(false);
	finished = $state(false);
	total = $state(0);
	processed = $state(0);
	imported = $state(0);
	skippedDuplicate = $state(0);
	failedCount = $state(0);
	failures = $state<ImportFailure[]>([]);
	cancelled = $state(false);

	started(total: number) {
		this.running = true;
		this.finished = false;
		this.cancelled = false;
		this.total = total;
		this.processed = 0;
		this.imported = 0;
		this.skippedDuplicate = 0;
		this.failedCount = 0;
		this.failures = [];
	}

	progress(p: ImportProgress) {
		this.processed = p.processed;
		this.total = p.total;
		this.imported = p.imported;
		this.skippedDuplicate = p.skipped_duplicate;
		this.failedCount = p.failed;
	}

	finish(summary: ImportFinished) {
		this.running = false;
		this.finished = true;
		this.total = summary.total;
		this.imported = summary.imported;
		this.skippedDuplicate = summary.skipped_duplicate;
		this.failedCount = summary.failed.length;
		this.failures = summary.failed;
		this.cancelled = summary.cancelled;
	}

	/** Clears a finished run's summary so the dialog can start fresh \u2014
	 *  doesn't touch anything while `running`. */
	dismiss() {
		if (this.running) return;
		this.finished = false;
		this.failures = [];
	}
}

export const importStore = new ImportStore();
