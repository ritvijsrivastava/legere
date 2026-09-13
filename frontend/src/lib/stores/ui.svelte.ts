const TOAST_DURATION_MS = 4000;

class UiStore {
	addSourceOpen = $state(false);
	importDialogOpen = $state(false);
	syncing = $state(false);
	toastMessage = $state<string | null>(null);
	private toastTimer: ReturnType<typeof setTimeout> | null = null;

	openAddSource() {
		this.addSourceOpen = true;
	}
	closeAddSource() {
		this.addSourceOpen = false;
	}

	openImportDialog() {
		this.importDialogOpen = true;
	}
	closeImportDialog() {
		this.importDialogOpen = false;
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
