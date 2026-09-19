import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import type {
	ArticleDetail,
	ArticleImportPreview,
	ArticlePage,
	ArticlePageRequest,
	ArticleSummary,
	CaptureJob,
	Category,
	ExportResult,
	ImportPreview,
	NamedCount,
	ReadingOverrides,
	Settings,
	Source,
	SourceArticlePreview,
	SourcesImportSummary,
	SourceType,
	SyncResult,
	TagFacetRequest
} from './types';

export { open as pickCsvFile, save as saveCsvFile } from '@tauri-apps/plugin-dialog';
/** Opens the OS file manager with `path` selected — backs "Reveal in
 *  folder" after an export (see `ExportResultCard.svelte`). */
export { revealItemInDir } from '@tauri-apps/plugin-opener';

/** Writes `contents` to `path`, overwriting any existing file — used to
 *  export data (e.g. failed Raindrop import rows as CSV) to a location
 *  picked via `saveCsvFile`. */
export function writeTextFile(path: string, contents: string): Promise<void> {
	return invoke<void>('write_text_file', { path, contents });
}

/** Reads a whole text file back — used by `ExportResultCard`'s "Save a
 *  copy..." action to hand an already-written export to a location the
 *  user picks, without re-running the export a second time. */
export function readTextFile(path: string): Promise<string> {
	return invoke<string>('read_text_file', { path });
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

export function countUncategorized(): Promise<number> {
	return invoke<number>('count_uncategorized');
}

export function listTags(): Promise<NamedCount[]> {
	return invoke<NamedCount[]>('list_tags');
}

/** Same shape as `listTags`, but narrowed to tags that co-occur with
 *  `request`'s already-selected category/tags — the sidebar's own facet
 *  narrowing, not the `/tags` management page (which always wants the
 *  unfiltered `listTags`). */
export function listTagsFiltered(request: TagFacetRequest): Promise<NamedCount[]> {
	return invoke<NamedCount[]>('list_tags_filtered', { request });
}

/** Renames a tag everywhere it's used. Merges into `next` (de-duped) on
 *  any article that already carries both. */
export function renameTag(old: string, next: string): Promise<void> {
	return invoke<void>('rename_tag', { old, new: next });
}

/** Removes a tag from every article that carries it. Articles themselves
 *  are never deleted. */
export function deleteTag(tag: string): Promise<void> {
	return invoke<void>('delete_tag', { tag });
}

/** Real, user-managed categories stored in the `categories` table. */
export function getCategories(): Promise<Category[]> {
	return invoke<Category[]>('get_categories');
}

export function createCategory(name: string): Promise<Category> {
	return invoke<Category>('create_category', { name });
}

export function renameCategory(id: string, name: string): Promise<Category> {
	return invoke<Category>('rename_category', { id, name });
}

/** `icon` is an id from the fixed category icon pack (`categoryIcons.ts`). */
export function setCategoryIcon(id: string, icon: string): Promise<Category> {
	return invoke<Category>('set_category_icon', { id, icon });
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

export function toggleFavorite(id: string): Promise<boolean> {
	return invoke<boolean>('toggle_favorite', { id });
}

export function saveReadingProgress(id: string, progress: number): Promise<void> {
	return invoke<void>('save_reading_progress', { id, progress });
}

/** Persists an article's whole reading-appearance override set (font
 *  size, text width, line height, theme) — send every field, not just the
 *  one that changed; sending all `null` resets the article back to
 *  tracking the global settings. */
export function setReadingOverrides(id: string, overrides: ReadingOverrides): Promise<void> {
	return invoke<void>('set_reading_overrides', { id, overrides });
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

/** Sniffs whether `value` is a feed or a plain article URL and captures
 *  it in the background — the single entry point behind the "Add a
 *  source" dialog. Returns as soon as the job is queued; track its
 *  outcome via `listCaptureJobs`/the `capture:*` events, not this
 *  promise's resolution. */
export function addSourceBackground(value: string): Promise<CaptureJob> {
	return invoke<CaptureJob>('add_source_background', { value });
}

/** Every in-flight or failed background capture (never a succeeded one
 *  — see `CaptureJob`). */
export function listCaptureJobs(): Promise<CaptureJob[]> {
	return invoke<CaptureJob[]>('list_capture_jobs');
}

export function retryCaptureJob(id: string): Promise<void> {
	return invoke<void>('retry_capture_job', { id });
}

export function dismissCaptureJob(id: string): Promise<void> {
	return invoke<void>('dismiss_capture_job', { id });
}

/** Aborts a still-running capture outright — safe at any point in the
 *  pipeline, see the backend's `capture_jobs::CaptureJobs::cancel`. */
export function cancelCaptureJob(id: string): Promise<void> {
	return invoke<void>('cancel_capture_job', { id });
}

/** Up to `limit` most-recent articles from one source — backs the
 *  Sources page's recent-articles strip. */
export function listSourceRecentArticles(
	sourceId: string,
	limit: number
): Promise<SourceArticlePreview[]> {
	return invoke<SourceArticlePreview[]>('list_source_recent_articles', { sourceId, limit });
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

export function importRaindropCsv(path: string): Promise<void> {
	return invoke<void>('import_raindrop_csv', { path });
}

/** Cancels the in-flight import, if any; resolves `true` if there was one
 *  to cancel. In-flight fetches (up to the backend's small worker-pool
 *  size) are still allowed to finish — only new ones are stopped. */
export function cancelRaindropImport(): Promise<boolean> {
	return invoke<boolean>('cancel_raindrop_import');
}

/** Writes every article currently in the library to a CSV file under the
 *  app's own `exports/` folder (Legere's own shape, independent of the
 *  Raindrop importer above) and returns its path/filename. Synchronous —
 *  a plain DB read + file write, no network involved. */
export function exportArticlesCsv(): Promise<ExportResult> {
	return invoke<ExportResult>('export_articles_csv');
}

export function previewArticlesCsv(path: string): Promise<ArticleImportPreview> {
	return invoke<ArticleImportPreview>('preview_articles_csv', { path });
}

/** Starts a background import of Legere's own article CSV shape —
 *  progress tracked via `article_import:*` events (see
 *  `stores/articleImport.svelte.ts`), not this call's return value. */
export function importArticlesCsv(path: string): Promise<void> {
	return invoke<void>('import_articles_csv', { path });
}

export function cancelArticlesImport(): Promise<boolean> {
	return invoke<boolean>('cancel_articles_import');
}

/** Writes every current source to a CSV file under the app's own
 *  `exports/` folder. Synchronous, like `exportArticlesCsv`. */
export function exportSourcesCsv(): Promise<ExportResult> {
	return invoke<ExportResult>('export_sources_csv');
}

/** Reads `path` (a CSV in `exportSourcesCsv`'s own shape) and queues
 *  every feed URL not already present for background capture — tracked
 *  via the existing capture-jobs activity dock (`listCaptureJobs`), not a
 *  dedicated progress UI. */
export function importSourcesCsv(path: string): Promise<SourcesImportSummary> {
	return invoke<SourcesImportSummary>('import_sources_csv', { path });
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
