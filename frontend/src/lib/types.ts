export type SourceType = 'rss' | 'direct';
export type SourceStatus = 'active' | 'paused' | 'error';
export type FontSize = 'small' | 'medium' | 'large';
export type LibraryView = 'cards' | 'list';
export type ReaderMeasure = 'narrow' | 'default' | 'wide';
export type ReaderLeading = 'compact' | 'default' | 'airy';

/** `unread` -> `reading` on opening the reader (whether previously unread
 *  or read); `reading` -> `read` only via the manual "mark as read" action. */
export type ReadingState = 'unread' | 'reading' | 'read';

/** The full-page archive's server-side capture job status. Always `ready`
 *  for `archive_source: 'local_legacy'` articles. */
export type ArchiveStatus = 'pending' | 'ready' | 'failed';

/** `server`: captured by the archive server, its local copy evictable.
 *  `local_legacy`: captured before server-side archiving existed,
 *  permanently local, never evicted. */
export type ArchiveSource = 'server' | 'local_legacy';

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
	archive_status: ArchiveStatus;
	archive_source: ArchiveSource;
	/** Whether the full archive's bytes are cached on this device right
	 *  now — independent of `archive_status`, since a `ready` archive may
	 *  still have been evicted locally. */
	archive_available_locally: boolean;
}

export interface ArticleDetail extends ArticleSummary {
	link: string;
	content_html: string;
	/** The archived page's own ZIM entry path — see `api.zimUrl`. `null`
	 *  until the server-side capture reports a result. */
	zim_main_path: string | null;
	/** `false` when captured via the naive extraction fallback; the reader
	 *  defaults to the archived (Original) view for such articles once
	 *  it's available. */
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
	/** Base URL of the self-hosted `legere-server` archive server, e.g.
	 *  `http://192.168.1.10:8787`. Empty means archiving is unconfigured. */
	archive_server_url: string;
	archive_server_token: string;
}

export interface SyncResult {
	new_article_count: number;
}
