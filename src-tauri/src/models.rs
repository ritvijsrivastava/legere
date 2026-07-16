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
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_font_size: "medium".into(),
            default_library_view: "cards".into(),
            autosync: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub new_article_count: u32,
}
