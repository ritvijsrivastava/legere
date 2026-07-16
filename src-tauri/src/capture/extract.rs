use dom_smoothie::{Article, Readability};

/// Readable content pulled out of a fetched page, before sanitization.
pub struct Extracted {
    pub title: String,
    pub content_html: String,
    pub excerpt: String,
    pub hero_image_url: Option<String>,
    pub published_at: Option<String>,
    pub read_time_min: i64,
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
    }
}

/// Used when the page isn't readable enough for `dom_smoothie` to extract a
/// confident article (e.g. a listing page, or a layout it doesn't recognize).
fn naive_fallback(html: &str) -> Extracted {
    let title = extract_tag_text(html, "title").unwrap_or_else(|| "Untitled".to_string());
    let text_content = strip_tags(html);
    Extracted {
        title,
        read_time_min: read_time_minutes(&text_content),
        excerpt: truncate_excerpt(&text_content),
        content_html: html.to_string(),
        hero_image_url: None,
        published_at: None,
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
