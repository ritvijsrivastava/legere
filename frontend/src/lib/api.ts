import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import type {
	AddSourceAutoResult,
	ArticleDetail,
	ArticleSummary,
	Settings,
	Source,
	SourceType,
	SyncResult
} from './types';

export { open as pickCsvFile } from '@tauri-apps/plugin-dialog';

export function listArticles(): Promise<ArticleSummary[]> {
	return invoke<ArticleSummary[]>('list_articles');
}

export function getArticle(id: string): Promise<ArticleDetail> {
	return invoke<ArticleDetail>('get_article', { id });
}

/** Transitions an article into `reading` (from `unread` or `read`) and
 *  returns its refreshed detail. */
export function openForReading(id: string): Promise<ArticleDetail> {
	return invoke<ArticleDetail>('open_for_reading', { id });
}

/** Transitions an article into `read` — the only path there, always a
 *  manual action. */
export function markAsRead(id: string): Promise<void> {
	return invoke<void>('mark_as_read', { id });
}

export function toggleFavorite(id: string): Promise<boolean> {
	return invoke<boolean>('toggle_favorite', { id });
}

export function saveReadingProgress(id: string, progress: number): Promise<void> {
	return invoke<void>('save_reading_progress', { id, progress });
}

export function addDirectLinkArticle(url: string): Promise<ArticleSummary> {
	return invoke<ArticleSummary>('add_direct_link_article', { url });
}

export function deleteArticle(id: string): Promise<void> {
	return invoke<void>('delete_article', { id });
}

export function recaptureArticle(id: string): Promise<ArticleDetail> {
	return invoke<ArticleDetail>('recapture_article', { id });
}

export function listSources(): Promise<Source[]> {
	return invoke<Source[]>('list_sources');
}

export function addSource(sourceType: SourceType, value: string): Promise<Source> {
	return invoke<Source>('add_source', { sourceType, value });
}

/** Sniffs whether `value` is a feed or a plain article URL and adds it
 *  accordingly — the single entry point behind the "Add a source" dialog. */
export function addSourceAuto(value: string): Promise<AddSourceAutoResult> {
	return invoke<AddSourceAutoResult>('add_source_auto', { value });
}

export function toggleSourcePause(id: string): Promise<Source> {
	return invoke<Source>('toggle_source_pause', { id });
}

export function removeSource(id: string): Promise<void> {
	return invoke<void>('remove_source', { id });
}

export function syncSource(id: string): Promise<SyncResult> {
	return invoke<SyncResult>('sync_source', { id });
}

export function syncAll(): Promise<SyncResult> {
	return invoke<SyncResult>('sync_all');
}

/** Starts a background Raindrop.io CSV import and returns as soon as it's
 *  running — progress is tracked via the `import:*` events (see
 *  `lib/events.ts` and `stores/import.svelte.ts`), not this call's return
 *  value. Rejects if an import is already in flight. */
export function importRaindropCsv(path: string): Promise<void> {
	return invoke<void>('import_raindrop_csv', { path });
}

/** Cancels the in-flight import, if any; resolves `true` if there was one
 *  to cancel. In-flight fetches (up to the backend's small worker-pool
 *  size) are still allowed to finish — only new ones are stopped. */
export function cancelRaindropImport(): Promise<boolean> {
	return invoke<boolean>('cancel_raindrop_import');
}

export function getSettings(): Promise<Settings> {
	return invoke<Settings>('get_settings');
}

export function updateSettings(settings: Settings): Promise<void> {
	return invoke<void>('update_settings', { settings });
}

let dataDirPromise: Promise<string> | null = null;

function getDataDir(): Promise<string> {
	if (!dataDirPromise) {
		dataDirPromise = invoke<string>('get_data_dir');
	}
	return dataDirPromise;
}

/** Resolves a relative path like `hero_image_path` into a webview-loadable asset URL. */
export async function assetUrl(relativePath: string | null): Promise<string | null> {
	if (!relativePath) return null;
	const dataDir = await getDataDir();
	const separator = dataDir.endsWith('/') || dataDir.endsWith('\\') ? '' : '/';
	return convertFileSrc(`${dataDir}${separator}${relativePath}`);
}

/** The `legere-content:/` token prefix capture writes into readable-view
 *  `content_html` (see `capture::rewrite` on the Rust side) — a
 *  platform-neutral placeholder since the real `legere-content://` URL
 *  shape differs across desktop and Android. */
const CONTENT_TOKEN_PREFIX = 'legere-content:/';

/** Rewrites every `legere-content:/<id>/<path>` token in `html` into a
 *  real, platform-correct `legere-content://` URL. Call once on an
 *  article's `content_html` before rendering it with `{@html}`. */
export function resolveContentTokens(html: string): string {
	const base = convertFileSrc('', 'legere-content');
	return html.replaceAll(CONTENT_TOKEN_PREFIX, base);
}

/** Tauri command errors reject with `{kind, message}` (see the Rust side's
 *  `error::AppError`) rather than a JS `Error`, so `e.message`/`e instanceof
 *  Error` don't work on them. This is the one place that shape is parsed —
 *  every catch site should go through this instead of reading `e` directly. */
export function errorMessage(e: unknown): string {
	if (e && typeof e === 'object' && 'message' in e && typeof e.message === 'string') {
		return e.message;
	}
	if (e instanceof Error) return e.message;
	return String(e);
}
