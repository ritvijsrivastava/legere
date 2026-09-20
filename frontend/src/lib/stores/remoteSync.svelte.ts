import * as api from '../api';
import type { RemoteSyncConfig, RemoteSyncStatus } from '../types';

/** Cross-device sync's own config/status, separate from `settingsStore`
 *  (which is RSS autosync and every other local-only preference). Mirrors
 *  `settingsStore`'s shape, but `config` starts `null` (sync may never
 *  have been configured on this device at all, a real distinct state from
 *  "configured but disabled") rather than a hardcoded default object. */
class RemoteSyncStore {
	config = $state<RemoteSyncConfig | null>(null);
	status = $state<RemoteSyncStatus>({ last_synced_at: null, last_error: null });
	loaded = $state(false);
	/** Set for the duration of a "Sync now" call or a save that also
	 *  re-verifies the connection - the hourly background loop's own runs
	 *  don't set this, only ones this tab initiated, since a spinner tied
	 *  to a background timer firing every hour would be more distracting
	 *  than useful. Backend-initiated activity still updates `status`
	 *  once finished, via `registerBackendEvents`'s `remote-sync:*`
	 *  listeners. */
	busy = $state(false);

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
	 *  that inline rather than this store swallowing it. */
	async save(config: RemoteSyncConfig): Promise<RemoteSyncConfig> {
		this.busy = true;
		try {
			this.config = await api.saveRemoteSyncConfig(config);
			return this.config;
		} finally {
			this.busy = false;
		}
	}

	async testConnection(config: RemoteSyncConfig): Promise<boolean> {
		this.busy = true;
		try {
			return await api.testRemoteSyncConnection(config);
		} finally {
			this.busy = false;
		}
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
}

export const remoteSyncStore = new RemoteSyncStore();
