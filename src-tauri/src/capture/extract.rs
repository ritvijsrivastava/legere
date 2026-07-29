use dom_smoothie::{Article, Readability};

/// Readable content pulled out of a fetched page, before sanitization.
pub struct Extracted {
    pub title: String,
    pub content_html: String,
    pub excerpt: String,
    pub hero_image_url: Option<String>,
    pub published_at: Option<String>,
    pub read_time_min: i64,
    /// `false` when extraction fell back to [`naive_fallback`] — the
    /// reader should prefer the archived (original) view over this
    /// article's readable view when this is `false`, since the naive path
    /// can only approximate an article's actual content boundaries.
    pub extraction_confident: bool,
}

/// Runs `dom_smoothie`'s readability extraction over `html`. Falls back to a
/// naive title+body extraction if the page isn't "probably readable" or
/// parsing fails outright — a rough article beats no article for MVP.
pub fn extract(html: &str, url: &str) -> Extracted {
    if let Ok(mut readability) = Readability::new(html, Some(url), None)
        && let Ok(article) = readability.parse()
    {
        return from_article(article);
    }
    naive_fallback(html)
}

fn from_article(article: Article) -> Extracted {
    let content_html = article.content.to_string();
    let text_content = article.text_content.to_string();
    let excerpt = article
        .excerpt
        .filter(|e| !e.trim().is_empty())
        .unwrap_or_else(|| truncate_excerpt(&text_content));

    Extracted {
        title: article.title,
        read_time_min: read_time_minutes(&text_content),
        content_html,
        excerpt,
        hero_image_url: article.image,
        published_at: article.published_time,
        extraction_confident: true,
    }
}

/// Used when the page isn't readable enough for `dom_smoothie` to extract a
/// confident article (e.g. a listing page, or a layout it doesn't
/// recognize). Stores the page's `<body>` inner HTML rather than the whole
/// document — the whole document (head, nav, scripts, footer chrome)
/// isn't an article and was never safe to hand to the reader as one — but
/// even the body still carries a page's full navigational chrome, which is
/// why this path is marked low-confidence: the reader defaults such
/// articles to the archived view instead of this best-effort one.
fn naive_fallback(html: &str) -> Extracted {
    let title = extract_tag_text(html, "title").unwrap_or_else(|| "Untitled".to_string());
    let body_html = extract_tag_text(html, "body");
    let text_content = strip_tags(body_html.as_deref().unwrap_or(html));
    Extracted {
        title,
        read_time_min: read_time_minutes(&text_content),
        excerpt: truncate_excerpt(&text_content),
        content_html: body_html.unwrap_or_default(),
        hero_image_url: None,
        published_at: None,
        extraction_confident: false,
    }
}

fn extract_tag_text(html: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let start = html.find(&open)?;
    let after_open = html[start..].find('>')? + start + 1;
    let close = format!("</{tag}>");
    let end = html[after_open..].find(&close)? + after_open;
    Some(html[after_open..end].trim().to_string())
}

fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

fn truncate_excerpt(text: &str) -> String {
    let normalized: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= 200 {
        normalized
    } else {
        let truncated: String = normalized.chars().take(200).collect();
        format!("{}…", truncated.trim_end())
    }
}

fn read_time_minutes(text: &str) -> i64 {
    let word_count = text.split_whitespace().count() as i64;
    (word_count / 200).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn naive_fallback_stores_body_inner_html_not_the_whole_document() {
        let html = "<html><head><title>Listing Page</title><script>track()</script></head>\
                     <body><nav>Home</nav><main>Some content here.</main></body></html>";
        let extracted = naive_fallback(html);

        assert_eq!(extracted.title, "Listing Page");
        assert!(!extracted.extraction_confident);
        assert_eq!(
            extracted.content_html,
            "<nav>Home</nav><main>Some content here.</main>"
        );
        // The whole document (head/script included) must never end up as
        // content_html — the bug this fallback rewrite fixed.
        assert!(!extracted.content_html.contains("<script>"));
        assert!(!extracted.content_html.contains("<title>"));
    }

    #[test]
    fn naive_fallback_falls_back_to_untitled_and_empty_content_without_a_body() {
        let html = "<html><head><title></title></head></html>";
        let extracted = naive_fallback(html);

        assert_eq!(extracted.title, "");
        assert!(!extracted.extraction_confident);
        assert_eq!(extracted.content_html, "");
    }

    #[test]
    fn naive_fallback_defaults_title_when_missing_entirely() {
        let extracted = naive_fallback("<html><body><p>No title here.</p></body></html>");
        assert_eq!(extracted.title, "Untitled");
        assert_eq!(extracted.content_html, "<p>No title here.</p>");
    }
}
