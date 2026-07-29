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

export function markRead(id: string): Promise<void> {
	return invoke<void>('mark_read', { id });
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
