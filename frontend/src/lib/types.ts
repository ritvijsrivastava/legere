export type SourceType = 'rss' | 'direct';
export type SourceStatus = 'active' | 'paused' | 'error';
export type FontSize = 'small' | 'medium' | 'large';
export type LibraryView = 'cards' | 'list';
export type ReaderMeasure = 'narrow' | 'default' | 'wide';
export type ReaderLeading = 'compact' | 'default' | 'airy';
export type AppTheme = 'light' | 'dark';
/** `light` tracks the current `AppTheme` rather than forcing a literal
 *  light palette; `sepia`/`dark` are fixed overrides. */
export type ReaderTheme = 'light' | 'sepia' | 'dark';

/** `unread` -> `reading` on opening the reader (whether previously unread
 *  or read); `reading` -> `read` only via the manual "mark as read" action. */
export type ReadingState = 'unread' | 'reading' | 'read';

export interface ArticleSummary {
	id: string;
	title: string;
	source_name: string;
	source_type: SourceType;
	excerpt: string;
	hero_image_path: string | null;
	published_at: string | null;
	read_time_min: number;
	reading_state: ReadingState;
	favorited: boolean;
	/** 0.0-1.0 scroll fraction, for the library card's progress indicator. */
	reading_progress: number;
	/** From the source feed's `<category>` elements; always empty for
	 *  direct-link articles. */
	tags: string[];
}

export interface ArticleDetail extends ArticleSummary {
	link: string;
	content_html: string;
	/** `false` when captured via the naive extraction fallback; the readable
	 *  view may be lower quality for such an article. */
	extraction_confident: boolean;
}

export interface Source {
	id: string;
	name: string;
	source_type: SourceType;
	feed_url: string | null;
	status: SourceStatus;
	last_error: string | null;
	article_count: number;
	last_synced_at: string | null;
	created_at: string;
}

/** Result of `add_source_auto`: the backend fetches the submitted URL once
 *  and sniffs whether it's a feed or a plain page, so the frontend never
 *  asks the user to choose up front. */
export type AddSourceAutoResult =
	| { kind: 'rss'; value: Source }
	| { kind: 'direct'; value: ArticleSummary };

export interface Settings {
	default_font_size: FontSize;
	default_library_view: LibraryView;
	autosync: boolean;
	/** Reader body font size in px (16-22) — the "Aa" popover's size step. */
	reader_font_size: number;
	reader_measure: ReaderMeasure;
	reader_leading: ReaderLeading;
	app_theme: AppTheme;
	reader_theme: ReaderTheme;
}

export interface SyncResult {
	new_article_count: number;
}

/** Keyset-pagination request for `list_articles_page` — mirrors the
 *  Rust `ArticlePageRequest`. `cursor_fetched_at`/`cursor_id` are both
 *  `null` for the first page; otherwise both come from the last item of
 *  the previously loaded page. Filters are applied server-side. */
export interface ArticlePageRequest {
	cursor_fetched_at: string | null;
	cursor_id: string | null;
	limit: number;
	search: string | null;
	source_name: string | null;
	/** Any-of match (OR semantics) against an article's tags. */
	tags: string[];
	favorited_only: boolean;
}

export interface ArticlePage {
	items: ArticleSummary[];
	/** Whether another page exists beyond `items` for the same filters. */
	has_more: boolean;
	/** `[fetched_at, id]` of `items`' last row — feed straight back in as
	 *  the next request's cursor. `null` exactly when `has_more` is false.
	 *  `ArticleSummary` doesn't carry `fetched_at` itself, so this is the
	 *  only way to form the next request. */
	next_cursor: [string, string] | null;
}

/** `[name, count]` pairs, as returned by `list_categories`/`list_tags`. */
export type NamedCount = [string, number];

export interface ImportFailure {
	url: string;
	title: string;
	error: string;
}

export interface ImportProgress {
	processed: number;
	total: number;
	imported: number;
	skipped_duplicate: number;
	failed: number;
}

export interface ImportFinished {
	total: number;
	imported: number;
	skipped_duplicate: number;
	failed: ImportFailure[];
	/** `true` if `cancel_raindrop_import` stopped the run early — the counts
	 *  above still reflect whatever completed before that point. */
	cancelled: boolean;
}
