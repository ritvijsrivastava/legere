use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleSummary {
    pub id: String,
    pub title: String,
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
    /// From the source feed's `<category>` elements (`feed_rs`); always
    /// empty for direct-link articles, which have no feed to draw from.
    pub tags: Vec<String>,
    /// The article's original external URL — the library card/row shows
    /// just its host (e.g. `example.com`), not the full URL, which is why
    /// this is included here rather than requiring a separate detail
    /// fetch.
    pub link: String,
    /// Name of the article's folder category (via `articles.category_id`),
    /// `None` when uncategorized. Resolved server-side so the label can't
    /// drift from a stale/unloaded client-side category list.
    pub category_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleDetail {
    pub id: String,
    pub title: String,
    pub source_type: String,
    pub excerpt: String,
    pub hero_image_path: Option<String>,
    pub published_at: Option<String>,
    pub read_time_min: i64,
    pub reading_state: String,
    pub favorited: bool,
    pub link: String,
    pub content_html: String,
    /// `false` when captured via the naive extraction fallback — the
    /// reader may want to surface that the readable view is lower
    /// confidence.
    pub extraction_confident: bool,
    pub reading_progress: f64,
    pub tags: Vec<String>,
    /// Name of the article's folder category, `None` when uncategorized —
    /// shown in the reader's byline in place of the old, purposeless
    /// `source_name` provenance label.
    pub category_name: Option<String>,
    /// This article's reading-appearance overrides, each `None` where it
    /// instead follows the global `Settings` value — see `ReadingOverrides`.
    pub overrides: ReadingOverrides,
}

/// Per-article overrides of the global reading-appearance settings
/// (`Settings::reader_font_size`/`reader_measure`/`reader_leading`/`app_theme`),
/// one field per axis, `None` meaning "no override, use the global value".
/// Set via `commands::articles::set_reading_overrides` from the reader's
/// "Aa" popover; resetting an article clears all four back to `None` in
/// one call rather than requiring four separate ones.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReadingOverrides {
    pub font_size: Option<i64>,
    pub measure: Option<String>,
    pub leading: Option<String>,
    /// `light` | `dark`, same two values as `Settings::app_theme`.
    pub theme: Option<String>,
}

/// Request for a page of [`ArticleSummary`] rows, keyset-paginated on
/// `(fetched_at, id)` DESC (see `db::queries::list_articles_page`). The
/// same request also carries every filter predicate the library/favorites
/// views support (search text, category, tags) — these are applied
/// server-side rather than over an in-memory array, since once paginated
/// the frontend no longer holds the whole table to filter locally.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArticlePageRequest {
    /// `None` on both fields requests the first page; otherwise both must
    /// be `Some`, taken verbatim from the previous page's last item.
    pub cursor_fetched_at: Option<String>,
    pub cursor_id: Option<String>,
    pub limit: i64,
    pub search: Option<String>,
    /// Real category/folder id. The reserved `__uncategorized__` value
    /// filters articles whose nullable `category_id` is NULL.
    pub category_id: Option<String>,
    /// Matches articles tagged with *any* of these (OR semantics) —
    /// mirrors the sidebar's multi-select tag filter.
    pub tags: Vec<String>,
    pub favorited_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticlePage {
    pub items: Vec<ArticleSummary>,
    /// Whether another page exists beyond `items` for the same filters —
    /// determined by fetching one extra row server-side, not by comparing
    /// `items.len()` to the requested limit.
    pub has_more: bool,
    /// `(fetched_at, id)` of `items`' last row, ready to feed straight
    /// back in as the next request's `cursor_fetched_at`/`cursor_id` —
    /// `Some` exactly when `has_more` is true. `ArticleSummary` itself
    /// doesn't carry `fetched_at`, so the frontend has no other way to
    /// form the next cursor.
    pub next_cursor: Option<(String, String)>,
}

/// A user-managed, flat category ("folder") an article can belong to.
/// `article_count` is computed at query time, not stored, and can
/// legitimately be `0` — a category persists after its last article is
/// reassigned elsewhere (see `db::schema`'s `V9` migration doc comment).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub article_count: i64,
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
    /// `light` | `dark` — the single app-wide color scheme, covering both
    /// the app chrome and the reader. An individual article can override
    /// just its own reader view via `ReadingOverrides::theme`.
    pub app_theme: String,
    /// How many Raindrop CSV import rows (`sources::raindrop_import::run_import`)
    /// capture concurrently. Clamped to 5–10 wherever it's read or written
    /// (`queries::get_settings`/`update_settings`) rather than trusted as
    /// free-form input — low enough to not hammer every site in an export
    /// at once, high enough to matter for a multi-thousand-row one.
    pub import_concurrency: i64,
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
            app_theme: "dark".into(),
            import_concurrency: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub new_article_count: u32,
}

/// Result of `add_source_auto`, which sniffs a user-submitted URL rather
/// than asking them to pick RSS-feed vs. article-link up front: a feed
/// registers a recurring `Source`, anything else is captured once as a
/// standalone article. `kind` lets the frontend branch (navigate into the
/// reader for `direct`, stay on the sources list for `rss`) without
/// re-deriving it from shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum AddSourceAutoResult {
    Rss(Source),
    Direct(ArticleSummary),
}

/// Metadata about an available update, sent to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
    pub date: Option<String>,
}
