export type SourceType = 'rss' | 'mail' | 'direct';
export type SourceStatus = 'active' | 'paused' | 'error';
export type FontSize = 'small' | 'medium' | 'large';
export type LibraryView = 'cards' | 'list';
export type ReaderMeasure = 'narrow' | 'default' | 'wide';
export type ReaderLeading = 'compact' | 'default' | 'airy';

export interface ArticleSummary {
	id: string;
	title: string;
	source_name: string;
	source_type: SourceType;
	excerpt: string;
	hero_image_path: string | null;
	published_at: string | null;
	read_time_min: number;
	unread: boolean;
	favorited: boolean;
	/** 0.0-1.0 scroll fraction, for the library card's progress indicator. */
	reading_progress: number;
}

export interface ArticleDetail extends ArticleSummary {
	link: string;
	content_html: string;
	/** The archived page's own ZIM entry path — see `api.zimUrl`. */
	zim_main_path: string;
	/** `false` when captured via the naive extraction fallback; the reader
	 *  defaults to the archived (Original) view for such articles. */
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

export interface Settings {
	default_font_size: FontSize;
	default_library_view: LibraryView;
	autosync: boolean;
	/** Reader body font size in px (16-22) — the "Aa" popover's size step. */
	reader_font_size: number;
	reader_measure: ReaderMeasure;
	reader_leading: ReaderLeading;
}

export interface SyncResult {
	new_article_count: number;
}
