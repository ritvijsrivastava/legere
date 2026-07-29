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
    pub unread: bool,
    pub favorited: bool,
    /// 0.0-1.0 scroll fraction, for the library card's progress indicator.
    pub reading_progress: f64,
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
    pub unread: bool,
    pub favorited: bool,
    pub link: String,
    pub content_html: String,
    /// The archived page's own ZIM entry path, for the reader's "Original"
    /// view: `convertFileSrc(`${id}/${zim_main_path}`, 'zim')`.
    pub zim_main_path: String,
    /// `false` when captured via the naive extraction fallback — the
    /// reader defaults to the archived view for such articles.
    pub extraction_confident: bool,
    pub reading_progress: f64,
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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub new_article_count: u32,
}
