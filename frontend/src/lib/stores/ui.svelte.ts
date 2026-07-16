class UiStore {
	addSourceOpen = $state(false);

	openAddSource() {
		this.addSourceOpen = true;
	}
	closeAddSource() {
		this.addSourceOpen = false;
	}
}

export const uiStore = new UiStore();
