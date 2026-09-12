import * as api from '../api';
import type { AddSourceAutoResult, Source, SourceType, SyncResult } from '../types';

class SourcesStore {
	items = $state<Source[]>([]);
	loading = $state(false);

	async refresh() {
		this.loading = true;
		try {
			this.items = await api.listSources();
		} finally {
			this.loading = false;
		}
	}

	async add(sourceType: SourceType, value: string): Promise<Source> {
		const source = await api.addSource(sourceType, value);
		this.items = [source, ...this.items];
		return source;
	}

	/** Backs the "Add a source" dialog's single auto-detecting field. Only
	 *  the `rss` branch touches this store's list — a `direct` result is a
	 *  one-shot article with no recurring source row to track. */
	async addAuto(value: string): Promise<AddSourceAutoResult> {
		const result = await api.addSourceAuto(value);
		if (result.kind === 'rss') {
			this.items = [result.value, ...this.items];
		}
		return result;
	}

	async togglePause(id: string) {
		const updated = await api.toggleSourcePause(id);
		this.items = this.items.map((s) => (s.id === id ? updated : s));
	}

	async remove(id: string) {
		await api.removeSource(id);
		this.items = this.items.filter((s) => s.id !== id);
	}

	async syncOne(id: string): Promise<SyncResult> {
		const result = await api.syncSource(id);
		await this.refresh();
		return result;
	}

	async syncAll(): Promise<SyncResult> {
		const result = await api.syncAll();
		await this.refresh();
		return result;
	}
}

export const sourcesStore = new SourcesStore();
