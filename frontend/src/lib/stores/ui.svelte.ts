const TOAST_DURATION_MS = 4000;

class UiStore {
	addSourceOpen = $state(false);
	captureJobsOpen = $state(false);
	importDialogOpen = $state(false);
	importArticlesDialogOpen = $state(false);
	importSourcesDialogOpen = $state(false);
	deleteAllArticlesDialogOpen = $state(false);
	/** Set by any "move to category" trigger (article card/row, reader
	 *  overflow menu) — the single globally-mounted `MoveToCategoryDialog`
	 *  reads this rather than each call site owning its own dialog. */
	moveCategoryArticle = $state<{ id: string; title: string } | null>(null);
	syncing = $state(false);
	toastMessage = $state<string | null>(null);
	private toastTimer: ReturnType<typeof setTimeout> | null = null;
	private moveCategoryCallback: (() => void) | null = null;

	openAddSource() {
		this.addSourceOpen = true;
	}
	closeAddSource() {
		this.addSourceOpen = false;
	}

	openCaptureJobs() {
		this.captureJobsOpen = true;
	}
	closeCaptureJobs() {
		this.captureJobsOpen = false;
	}

	openImportDialog() {
		this.importDialogOpen = true;
	}
	closeImportDialog() {
		this.importDialogOpen = false;
	}

	openImportArticlesDialog() {
		this.importArticlesDialogOpen = true;
	}
	closeImportArticlesDialog() {
		this.importArticlesDialogOpen = false;
	}

	openImportSourcesDialog() {
		this.importSourcesDialogOpen = true;
	}
	closeImportSourcesDialog() {
		this.importSourcesDialogOpen = false;
	}

	openDeleteAllArticlesDialog() {
		this.deleteAllArticlesDialogOpen = true;
	}
	closeDeleteAllArticlesDialog() {
		this.deleteAllArticlesDialogOpen = false;
	}

	/** `onMoved` fires once, right after the article's category is actually
	 *  changed — callers use it to drop the article from a list that's
	 *  scoped to the category it just left (see `ArticleCollection`). */
	openMoveCategory(article: { id: string; title: string }, onMoved?: () => void) {
		this.moveCategoryArticle = article;
		this.moveCategoryCallback = onMoved ?? null;
	}
	closeMoveCategory() {
		this.moveCategoryArticle = null;
		this.moveCategoryCallback = null;
	}
	notifyArticleMoved() {
		this.moveCategoryCallback?.();
	}

	showToast(message: string) {
		this.toastMessage = message;
		if (this.toastTimer) clearTimeout(this.toastTimer);
		this.toastTimer = setTimeout(() => {
			this.toastMessage = null;
		}, TOAST_DURATION_MS);
	}
}

export const uiStore = new UiStore();
