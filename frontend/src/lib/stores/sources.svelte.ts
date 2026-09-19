import * as api from '../api';
import type { Source, SourceType, SyncResult } from '../types';

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
