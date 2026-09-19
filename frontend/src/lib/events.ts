import { listen } from '@tauri-apps/api/event';
import { libraryStatsStore } from './stores/libraryStats.svelte';
import { sourcesStore } from './stores/sources.svelte';
import { uiStore } from './stores/ui.svelte';
import { importStore } from './stores/import.svelte';
import { articleImportStore } from './stores/articleImport.svelte';
import { captureJobsStore } from './stores/captureJobs.svelte';
import type { CaptureSucceeded, ImportFinished, ImportProgress } from './types';

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
		libraryStatsStore.notifyChanged();
	});

	listen('source:changed', () => {
		sourcesStore.refresh();
	});

	listen('category:changed', () => {
		libraryStatsStore.notifyChanged();
	});

	listen<number>('import:started', (event) => {
		importStore.started(event.payload);
	});

	listen<ImportProgress>('import:progress', (event) => {
		importStore.progress(event.payload);
	});

	listen<ImportFinished>('import:finished', (event) => {
		importStore.finish(event.payload);
		if (event.payload.failed.length > 0) {
			uiStore.showToast(
				`Import finished with ${event.payload.failed.length} failed link${event.payload.failed.length === 1 ? '' : 's'}`
			);
		}
	});

	listen<number>('article_import:started', (event) => {
		articleImportStore.started(event.payload);
	});

	listen<ImportProgress>('article_import:progress', (event) => {
		articleImportStore.progress(event.payload);
	});

	listen<ImportFinished>('article_import:finished', (event) => {
		articleImportStore.finish(event.payload);
		if (event.payload.failed.length > 0) {
			uiStore.showToast(
				`Import finished with ${event.payload.failed.length} failed link${event.payload.failed.length === 1 ? '' : 's'}`
			);
		}
	});

	listen('capture:changed', () => {
		captureJobsStore.refresh();
	});

	listen<CaptureSucceeded>('capture:succeeded', (event) => {
		uiStore.showToast(
			event.payload.kind === 'rss'
				? `Following “${event.payload.title}”`
				: `Added “${event.payload.title}”`
		);
	});
}
