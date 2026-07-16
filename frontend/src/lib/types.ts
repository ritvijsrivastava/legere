export type SourceType = 'rss' | 'mail' | 'direct';
export type SourceStatus = 'active' | 'paused' | 'error';
export type FontSize = 'small' | 'medium' | 'large';
export type LibraryView = 'cards' | 'list';

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
}

export interface ArticleDetail extends ArticleSummary {
	link: string;
	content_html: string;
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
}

export interface SyncResult {
	new_article_count: number;
}
