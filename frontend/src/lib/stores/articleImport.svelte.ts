import type { ImportFailure, ImportFinished, ImportProgress } from '$lib/types';

/** Tracks Settings' "Import articles" (Legere's own CSV shape) lifecycle,
 *  driven by the backend's `article_import:*` events \u2014 a direct sibling
 *  of `importStore` (the Raindrop-migration importer), kept as a fully
 *  separate module-level store so the two can never bleed into each
 *  other's progress display even though their shapes match. */
class ArticleImportStore {
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

	dismiss() {
		if (this.running) return;
		this.finished = false;
		this.failures = [];
	}
}

export const articleImportStore = new ArticleImportStore();
