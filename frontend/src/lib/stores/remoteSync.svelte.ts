import * as api from '../api';
import type { RemoteSyncConfig, RemoteSyncProgress, RemoteSyncStatus } from '../types';

/** Cross-device sync's own config/status, separate from `settingsStore`
 *  (which is RSS autosync and every other local-only preference). Mirrors
 *  `settingsStore`'s shape, but `config` starts `null` (sync may never
 *  have been configured on this device at all, a real distinct state from
 *  "configured but disabled") rather than a hardcoded default object. */
class RemoteSyncStore {
	config = $state<RemoteSyncConfig | null>(null);
	status = $state<RemoteSyncStatus>({ last_synced_at: null, last_error: null });
	loaded = $state(false);
	/** Whether an actual, cancellable cross-device sync *pass* is in
	 *  flight right now - driven only by the backend's own
	 *  `remote-sync:started/finished/error/cancelled` events
	 *  (`registerBackendEvents`) plus `syncNow`'s own optimistic flip, so
	 *  it fires for a background-loop-initiated pass too, not just ones
	 *  this tab asked for. Backs the Settings screen's Cancel button and
	 *  progress bar - it must *not* also cover `save`/`testConnection`
	 *  below: those are a config-save round trip, not a sync pass, and
	 *  flipping this for them used to make the Cancel button (wrongly)
	 *  flash on for e.g. just changing the sync-frequency stepper. */
	busy = $state(false);
	/** The most recent `remote-sync:progress` event, or `null` between
	 *  phases/before a sync starts. Set by `lib/events.ts`'s listener,
	 *  cleared whenever a sync starts or finishes/errors so a stale phase
	 *  never lingers on screen. */
	progress = $state<RemoteSyncProgress | null>(null);

	async refresh() {
		this.config = await api.getRemoteSyncConfig();
		this.status = await api.getRemoteSyncStatus();
		this.loaded = true;
	}

	async refreshStatus() {
		this.status = await api.getRemoteSyncStatus();
	}

	/** Throws on failure (an invalid bucket, or one that fails the
	 *  conditional-write check) - the caller (the config dialog) surfaces
	 *  that inline rather than this store swallowing it. Deliberately
	 *  doesn't touch `busy`: saving a config (even just the sync-frequency
	 *  stepper) isn't a sync pass, and the dialog already tracks its own
	 *  `saving` state for its own button/spinner. */
	async save(config: RemoteSyncConfig): Promise<RemoteSyncConfig> {
		this.config = await api.saveRemoteSyncConfig(config);
		return this.config;
	}

	/** Same `busy` exclusion as `save` above - the dialog tracks its own
	 *  `testState` for this. */
	async testConnection(config: RemoteSyncConfig): Promise<boolean> {
		return await api.testRemoteSyncConnection(config);
	}

	async syncNow() {
		this.busy = true;
		try {
			await api.remoteSyncNow();
			await this.refreshStatus();
		} finally {
			this.busy = false;
		}
	}

	/** Requests cancellation of whatever pass is currently running. Doesn't
	 *  itself flip `busy`/`progress` back — the backend's
	 *  `remote-sync:cancelled` event (see `lib/events.ts`) does that once
	 *  the pass actually stops, which may be a moment after this resolves. */
	async cancel() {
		await api.cancelRemoteSync();
	}
}

export const remoteSyncStore = new RemoteSyncStore();
