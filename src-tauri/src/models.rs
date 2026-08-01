use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleSummary {
    pub id: String,
    pub title: String,
    pub source_name: String,
    pub source_type: String,
    pub excerpt: String,
    pub hero_image_path: Option<String>,
    pub published_at: Option<String>,
    pub read_time_min: i64,
    /// `unread` | `reading` | `read`. Opening the reader transitions
    /// `unread`/`read` -> `reading`; only the manual "mark as read" action
    /// transitions `reading` -> `read`.
    pub reading_state: String,
    pub favorited: bool,
    /// 0.0-1.0 scroll fraction, for the library card's progress indicator.
    pub reading_progress: f64,
    /// `pending` | `ready` | `failed` — the server-side full-page capture
    /// job's status. Always `ready` for `local_legacy` articles.
    pub archive_status: String,
    /// `server` (captured by the archive server, evictable locally) or
    /// `local_legacy` (captured before server-side archiving existed,
    /// permanently local, never evicted).
    pub archive_source: String,
    /// Whether the full archive's bytes are cached on this device right
    /// now — independent of `archive_status`, since a `ready` archive may
    /// still have been evicted locally.
    pub archive_available_locally: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleDetail {
    pub id: String,
    pub title: String,
    pub source_name: String,
    pub source_type: String,
    pub excerpt: String,
    pub hero_image_path: Option<String>,
    pub published_at: Option<String>,
    pub read_time_min: i64,
    pub reading_state: String,
    pub favorited: bool,
    pub link: String,
    pub content_html: String,
    /// The archived page's own ZIM entry path, for the reader's "Original"
    /// view: `convertFileSrc(`${id}/${zim_main_path}`, 'zim')`. `None`
    /// until the server-side capture reports a result.
    pub zim_main_path: Option<String>,
    /// `false` when captured via the naive extraction fallback — the
    /// reader defaults to the archived view for such articles.
    pub extraction_confident: bool,
    pub reading_progress: f64,
    pub archive_status: String,
    pub archive_source: String,
    pub archive_available_locally: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub source_type: String,
    pub feed_url: Option<String>,
    pub status: String,
    pub last_error: Option<String>,
    pub article_count: i64,
    pub last_synced_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub default_font_size: String,
    pub default_library_view: String,
    pub autosync: bool,
    /// Reader body font size in px (16-22), replacing the old S/M/L
    /// `default_font_size` scale for the reader specifically — an "Aa"
    /// popover controls this directly rather than three fixed steps.
    pub reader_font_size: i64,
    pub reader_measure: String,
    pub reader_leading: String,
    /// Base URL of the self-hosted `legere-server` archive server, e.g.
    /// `http://192.168.1.10:8787`. Empty means archiving is unconfigured —
    /// the archive reconciler skips its poll tick silently in that case.
    pub archive_server_url: String,
    pub archive_server_token: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_font_size: "medium".into(),
            default_library_view: "cards".into(),
            autosync: true,
            reader_font_size: 19,
            reader_measure: "default".into(),
            reader_leading: "default".into(),
            archive_server_url: String::new(),
            archive_server_token: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub new_article_count: u32,
}
