import { listen } from '@tauri-apps/api/event';
import { articlesStore } from './stores/articles.svelte';
import { sourcesStore } from './stores/sources.svelte';
import { uiStore } from './stores/ui.svelte';

interface SyncFinishedPayload {
	new_article_count: number;
	errors: { source_id: string; message: string }[];
}

/** Registers listeners for the backend's `AppHandle::emit` events (see
 *  `src-tauri/src/events.rs`) and dispatches to whichever store's data
 *  might now be stale. Coarse-grained refetch rather than payload-carried
 *  deltas — the dataset is small, and this is what fixes the original
 *  bug: nothing told the library to refresh after a background autosync
 *  tick, so newly captured articles were invisible until restart.
 *
 *  Called once from the root layout; listeners live for the app's whole
 *  lifetime, so there's no matching unlisten. */
export function registerBackendEvents() {
	listen('sync:started', () => {
		uiStore.syncing = true;
	});

	listen<SyncFinishedPayload>('sync:finished', (event) => {
		uiStore.syncing = false;
		const { errors } = event.payload;
		if (errors.length > 0) {
			uiStore.showToast(
				errors.length === 1
					? `Sync failed: ${errors[0].message}`
					: `Sync failed for ${errors.length} sources`
			);
		}
	});

	listen('articles:changed', () => {
		articlesStore.refresh();
	});

	listen('source:changed', () => {
		sourcesStore.refresh();
	});
}
