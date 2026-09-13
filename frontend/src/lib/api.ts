import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import type {
	AddSourceAutoResult,
	ArticleDetail,
	ArticlePage,
	ArticlePageRequest,
	ArticleSummary,
	Category,
	FolderResolution,
	ImportPreview,
	NamedCount,
	Settings,
	Source,
	SourceType,
	SyncResult
} from './types';

export { open as pickCsvFile, save as saveCsvFile } from '@tauri-apps/plugin-dialog';

/** Writes `contents` to `path`, overwriting any existing file — used to
 *  export data (e.g. failed Raindrop import rows as CSV) to a location
 *  picked via `saveCsvFile`. */
export function writeTextFile(path: string, contents: string): Promise<void> {
	return invoke<void>('write_text_file', { path, contents });
}

export function listArticlesPage(request: ArticlePageRequest): Promise<ArticlePage> {
	return invoke<ArticlePage>('list_articles_page', { request });
}

export function countAllArticles(): Promise<number> {
	return invoke<number>('count_all_articles');
}

export function countUnread(): Promise<number> {
	return invoke<number>('count_unread');
}

export function countFavorited(): Promise<number> {
	return invoke<number>('count_favorited');
}

export function listCategories(): Promise<NamedCount[]> {
	return invoke<NamedCount[]>('list_categories');
}

export function listTags(): Promise<NamedCount[]> {
	return invoke<NamedCount[]>('list_tags');
}

/** Real, user-managed categories (`categories` table) — named
 *  `getCategories` on the frontend too, to keep it visually distinct
 *  from the soon-to-be-superseded `listCategories` above. */
export function getCategories(): Promise<Category[]> {
	return invoke<Category[]>('get_categories');
}

export function createCategory(name: string): Promise<Category> {
	return invoke<Category>('create_category', { name });
}

export function renameCategory(id: string, name: string): Promise<Category> {
	return invoke<Category>('rename_category', { id, name });
}

export function deleteCategory(id: string): Promise<void> {
	return invoke<void>('delete_category', { id });
}

/** `categoryId: null` clears an article's category (Uncategorized). */
export function setArticleCategory(id: string, categoryId: string | null): Promise<void> {
	return invoke<void>('set_article_category', { id, categoryId });
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

/** Replaces an article's whole tag set. The backend normalizes to
 *  lowercase/trimmed/deduped regardless of what's passed in and returns
 *  the tags actually stored, for the caller to reconcile local state with. */
export function setArticleTags(id: string, tags: string[]): Promise<string[]> {
	return invoke<string[]>('set_article_tags', { id, tags });
}

export function addDirectLinkArticle(url: string): Promise<ArticleSummary> {
	return invoke<ArticleSummary>('add_direct_link_article', { url });
}

export function deleteArticle(id: string): Promise<void> {
	return invoke<void>('delete_article', { id });
}

/** Permanently deletes every article and its files. Sources are kept. */
export function deleteAllArticles(): Promise<void> {
	return invoke<void>('delete_all_articles');
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
export function previewRaindropCsv(path: string): Promise<ImportPreview> {
	return invoke<ImportPreview>('preview_raindrop_csv', { path });
}

export function importRaindropCsv(
	path: string,
	resolutions: FolderResolution[] = []
): Promise<void> {
	return invoke<void>('import_raindrop_csv', { path, resolutions });
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
