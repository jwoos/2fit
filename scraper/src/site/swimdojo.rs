//! swimdojo.com scraper, via the Squarespace RSS feed.
//!
//! Why RSS instead of HTML scraping (evidence, 2026-09-08):
//!
//! - The listing page's filter UI is client-rendered JS, but the feed
//!   honors `?tag=<exact-tag>`, `?author=<id>`, and `?offset=<epoch-ms>`
//!   cursor pagination server-side (each page holds 20 items; a cursor of
//!   an item's exact `pubDate`-as-ms repeats that page: skip items with
//!   `published <= cursor`).
//! - Each item's `content:encoded` carries the full workout body HTML.
//!   Normalizing it (drop `<figure>` media blocks, take `<p>` blocks,
//!   `<br>` → newline, strip tags, unescape entities, cut at `TOTAL:`)
//!   reproduces the parser fixtures: box-crab and sea-otter byte-exact,
//!   goblin-shark modulo the site's `'` (kept) vs the fixture's `'`
//!   (hand-normalized in turn 5; both parse identically).
//! - Plain-HTML detail pages are the fallback (`fetch`), reading the first
//!   `div.sqs-html-content` inside `div[data-layout-label="Post Body"]`
//!   (verified on goblin-shark: same normalized text as the feed).
//! - `robots.txt` disallows `?tag=`/`?author=` for crawlers; `format=rss`
//!   is not disallowed. Scrape politely: browser UA, 1 req/s.

use super::traits::{ListFilter, ListingItem, ScrapedWorkout, Site, SiteError};
use std::time::Duration;

/// Base URL of the swimdojo workouts collection.
pub const FEED_BASE: &str = "https://www.swimdojo.com/workouts";

/// Browser-ish user agent (Squarespace may block non-browser UAs).
pub const USER_AGENT: &str = "Mozilla/5.0 (2fit-scrape)";

/// Pause between paginated feed requests (polite crawl).
pub const PAGE_DELAY: Duration = Duration::from_secs(1);

/// A swimdojo.com workout site.
#[derive(Debug, Clone)]
pub struct Swimdojo {
    agent: ureq::Agent,
}

impl Default for Swimdojo {
    fn default() -> Self {
        Self::new()
    }
}

impl Swimdojo {
    /// Build a scraper with the default agent (browser UA, 10 s timeout).
    pub fn new() -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .user_agent(USER_AGENT)
            .timeout_global(Some(Duration::from_secs(10)))
            .build()
            .into();
        Self { agent }
    }

    /// Feed URL for `filter`: `?format=rss` plus server-side params.
    /// `offset` is a pagination cursor, threaded separately by [`Site::list`].
    pub fn feed_url(filter: &ListFilter, offset: Option<i64>) -> String {
        let mut url = format!("{FEED_BASE}?format=rss");
        if let Some(tag) = &filter.tag {
            url.push_str(&format!("&tag={tag}"));
        }
        if let Some(author) = &filter.author {
            url.push_str(&format!("&author={author}"));
        }
        if let Some(offset) = offset {
            url.push_str(&format!("&offset={offset}"));
        }
        url
    }

    fn get(&self, url: &str) -> Result<String, SiteError> {
        self.agent
            .get(url)
            .call()
            .map_err(|e| SiteError(format!("GET {url}: {e}")))?
            .body_mut()
            .read_to_string()
            .map_err(|e| SiteError(format!("reading {url}: {e}")))
    }

    /// Parse one RSS document into items; `want` caps how many to keep.
    pub fn parse_feed(xml: &str, want: usize) -> Result<Vec<ListingItem>, SiteError> {
        let channel =
            rss::Channel::read_from(xml.as_bytes()).map_err(|e| SiteError(e.to_string()))?;
        Ok(channel
            .items()
            .iter()
            .take(want)
            .map(ListingItem::from)
            .collect())
    }

    /// Parse one RSS document into `(item, body-html)` pairs.
    pub fn parse_feed_bodies(xml: &str) -> Result<Vec<(ListingItem, String)>, SiteError> {
        let channel =
            rss::Channel::read_from(xml.as_bytes()).map_err(|e| SiteError(e.to_string()))?;
        Ok(channel
            .items()
            .iter()
            .filter_map(|item| {
                let html = item.content.clone()?;
                Some((ListingItem::from(item), html))
            })
            .collect())
    }

    fn feed_items(&self, filter: &ListFilter, offset: Option<i64>) -> Result<String, SiteError> {
        let url = Self::feed_url(filter, offset);
        let xml = self.get(&url)?;
        Ok(xml)
    }
}

impl Site for Swimdojo {
    fn list(&self, filter: &ListFilter) -> Result<Vec<ListingItem>, SiteError> {
        let want = filter.limit.unwrap_or(usize::MAX);
        let mut out: Vec<ListingItem> = Vec::new();
        let mut cursor: Option<i64> = None;
        loop {
            let xml = self.feed_items(filter, cursor)?;
            let page = Self::parse_feed(&xml, want.saturating_sub(out.len()).max(1))?;
            if page.is_empty() {
                break;
            }
            let mut fresh = 0;
            for item in page {
                let seen = match (item.published, cursor) {
                    (Some(p), Some(c)) => p <= c,
                    _ => false,
                };
                if !seen {
                    if let Some(q) = &filter.query
                        && !matches_query(&item, q)
                    {
                        continue;
                    }
                    out.push(item);
                    fresh += 1;
                }
                if out.len() >= want {
                    break;
                }
            }
            if out.len() >= want {
                break;
            }
            let last = out.last().and_then(|i| i.published);
            match last {
                Some(p) if fresh > 0 => {
                    cursor = Some(p);
                    std::thread::sleep(PAGE_DELAY);
                }
                _ => break,
            }
        }
        Ok(out)
    }

    fn fetch(&self, url: &str) -> Result<ScrapedWorkout, SiteError> {
        let page = self.get(url)?;
        let (title, html) =
            detail_body(&page).ok_or_else(|| SiteError(format!("no workout body in {url}")))?;
        let body = normalize_body(&html);
        Ok(ScrapedWorkout {
            title,
            url: url.to_owned(),
            body,
        })
    }
}

impl From<&rss::Item> for ListingItem {
    /// Map a feed item: title/link/description verbatim; author from
    /// Dublin Core `creator` (the feed's `<author>`-ish field);
    /// `published` from RFC 2822 `pubDate` (seconds epoch).
    fn from(item: &rss::Item) -> Self {
        let author = item
            .dublin_core_ext
            .as_ref()
            .and_then(|dc| dc.creators().first().cloned());
        let published = item
            .pub_date
            .as_deref()
            .and_then(|d| chrono::DateTime::parse_from_rfc2822(d).ok())
            .map(|d| d.timestamp());
        Self {
            title: item.title.clone().unwrap_or_default(),
            url: item.link.clone().unwrap_or_default(),
            author,
            tags: item.categories().iter().map(|c| c.name.clone()).collect(),
            description: item.description.clone(),
            published,
        }
    }
}

/// Client-side free-text match over title/description/tags.
fn matches_query(item: &ListingItem, query: &str) -> bool {
    let q = query.to_ascii_lowercase();
    item.title.to_ascii_lowercase().contains(&q)
        || item
            .description
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .contains(&q)
        || item
            .tags
            .iter()
            .any(|t| t.to_ascii_lowercase().contains(&q))
}

/// Normalize workout body HTML into parser-ready notation text.
///
/// - drops `<figure>…</figure>` media/creature-writeup blocks (post-workout
///   prose lives there — verified across 68 items that nothing after the
///   first `<figure` is workout text, except one caption-less photo
///   (`Bobtail Squid`, dropped) and one stray caption
///   (`Pacific Spiny Lumpsucker`, pre-existing prose duplicated in the
///   body anyway);
/// - keeps `<p>` blocks (the workout lines), skipping the links-boilerplate
///   paragraph (`How To Read a Workout`);
/// - `<br>` → newline, other tags unwrapped, entities unescaped;
/// - cuts everything after the `TOTAL:` line (figure-adjacent prose was
///   already dropped; this is belt-and-braces).
pub fn normalize_body(html: &str) -> String {
    let pre = match html.find("<figure") {
        Some(i) => &html[..i],
        None => html,
    };
    let mut out = Vec::new();
    for block in paragraphs(pre) {
        if block.contains("How To Read a Workout") {
            continue;
        }
        let mut text = block.replace("<br />", "\n").replace("<br/>", "\n");
        text = text.replace("<br>", "\n");
        text = strip_tags(&text);
        let text = html_escape::decode_html_entities(&text)
            .replace('\u{a0}', " ")
            .trim()
            .to_owned();
        if !text.is_empty() {
            out.push(text);
        }
    }
    let joined = out.join("\n");
    let mut lines: Vec<&str> = joined.lines().collect();
    if let Some(i) = lines
        .iter()
        .position(|l| l.trim_start().to_ascii_lowercase().starts_with("total:"))
    {
        lines.truncate(i + 1);
    }
    let mut body = lines.join("\n");
    body = body
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    if !body.is_empty() {
        body.push('\n');
    }
    body
}

/// Inner HTML of each text block, in order: `<p>` paragraphs plus `<li>`
/// items (open-water workouts such as Hourglass Dolphin put drill bullets
/// in a `<ul>`; pool workouts are `<p>`-only). `<h3>` headings are kept as
/// their own line so the body preserves section structure.
fn paragraphs(html: &str) -> Vec<String> {
    blocks(html, "p")
        .into_iter()
        .chain(blocks(html, "li"))
        .chain(blocks(html, "h3"))
        .collect()
}

/// Inner HTML of each `<tag>…</tag>` block, in document order.
fn blocks(html: &str, tag: &str) -> Vec<String> {
    let close = format!("</{tag}>");
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find(&format!("<{tag}")) {
        let after_tag = match rest[start..].find('>') {
            Some(i) => start + i + 1,
            None => break,
        };
        let end = match rest[after_tag..].find(&close) {
            Some(i) => after_tag + i,
            None => break,
        };
        out.push(rest[after_tag..end].to_owned());
        rest = &rest[end + close.len()..];
    }
    out
}

/// Remove `<…>` tags, keeping inner text (links, `<em>`, `<strong>`).
fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// Title + body HTML of a plain-HTML detail page: the first
/// `div.sqs-html-content` inside `div[data-layout-label="Post Body"]`.
/// Falls back to the page `<title>` when no `<h1>` is present.
pub fn detail_body(page: &str) -> Option<(String, String)> {
    let section = page
        .find("data-layout-label=\"Post Body\"")
        .map(|i| &page[i..])?;
    let div = section.find("sqs-html-content").map(|i| &section[i..])?;
    let inner = div.find('>')?;
    let mut depth = 1;
    let mut i = inner + 1;
    while depth > 0 && i < div.len() {
        if div[i..].starts_with("<div") {
            depth += 1;
            i += "<div".len();
        } else if div[i..].starts_with("</div>") {
            depth -= 1;
            if depth == 0 {
                let html = &div[inner + 1..i];
                let title = detail_title(page);
                return Some((title, html.to_owned()));
            }
            i += "</div>".len();
        } else {
            i += div[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        }
    }
    None
}

fn detail_title(page: &str) -> String {
    if let Some(h1) = tag_text(page, "h1") {
        return h1;
    }
    tag_text(page, "title").unwrap_or_default()
}

fn tag_text(page: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let start = page.find(&open)?;
    let inner = page[start..].find('>')? + start + 1;
    let end = page[inner..].find(&format!("</{tag}>"))? + inner;
    let text = strip_tags(&page[inner..end]);
    let text = html_escape::decode_html_entities(&text).trim().to_owned();
    let text = text.split(['—', '|']).next().unwrap_or(&text).trim();
    (!text.is_empty()).then_some(text.to_owned())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
