const STORAGE_KEY = 'legere.sidebar.collapsed';

function readStored(): boolean {
	try {
		return typeof localStorage !== 'undefined' && localStorage.getItem(STORAGE_KEY) === '1';
	} catch {
		return false;
	}
}

/** Desktop sidebar collapsed/expanded. A per-device UI preference, so it
 *  lives in `localStorage` (read synchronously — no flash of the wrong
 *  state on launch) rather than the synced-adjacent `settings` table. */
class SidebarStore {
	collapsed = $state(readStored());

	toggle() {
		this.set(!this.collapsed);
	}

	set(value: boolean) {
		this.collapsed = value;
		try {
			localStorage.setItem(STORAGE_KEY, value ? '1' : '0');
		} catch {
			// Storage unavailable — the choice just won't survive a restart.
		}
	}
}

export const sidebarStore = new SidebarStore();
