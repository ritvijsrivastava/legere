import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import type {
	ArticleDetail,
	ArticleSummary,
	Settings,
	Source,
	SourceType,
	SyncResult
} from './types';

export function listArticles(): Promise<ArticleSummary[]> {
	return invoke<ArticleSummary[]>('list_articles');
}

export function getArticle(id: string): Promise<ArticleDetail> {
	return invoke<ArticleDetail>('get_article', { id });
}

/** Transitions an article into `reading` (from `unread` or `read`) and
 *  returns its refreshed detail. Also kicks off a background archive
 *  download server-side if the full archive is `ready` but not yet cached
 *  locally — see `commands::articles::open_for_reading` on the Rust side. */
export function openForReading(id: string): Promise<ArticleDetail> {
	return invoke<ArticleDetail>('open_for_reading', { id });
}

/** Transitions an article into `read` — the only path there, always a
 *  manual action. Evicts its locally cached full archive, if any; the
 *  server retains its own copy. */
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

/** Builds a webview-loadable URL for `path` inside an article's own ZIM
 *  archive, served by the `zim://` protocol handler registered in `lib.rs`. */
export function zimUrl(articleId: string, path: string): string {
	return convertFileSrc(`${articleId}/${path}`, 'zim');
}

/** The `legere-zim:/` token prefix capture writes into readable-view
 *  `content_html` (see `capture::rewrite` on the Rust side) — a
 *  platform-neutral placeholder since the real `zim://` URL shape differs
 *  across desktop and Android. */
const ZIM_TOKEN_PREFIX = 'legere-zim:/';

/** Rewrites every `legere-zim:/<id>/<path>` token in `html` into a real,
 *  platform-correct `zim://` URL. Call once on an article's `content_html`
 *  before rendering it with `{@html}`. */
export function resolveZimTokens(html: string): string {
	const base = convertFileSrc('', 'zim');
	return html.replaceAll(ZIM_TOKEN_PREFIX, base);
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
