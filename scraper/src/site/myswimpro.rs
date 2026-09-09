//! myswimpro.com blog workouts, via the WordPress REST API.
//!
//! Why the WP API instead of HTML scraping (evidence, 2026-09-09):
//!
//! - `blog.myswimpro.com` is WordPress: `/wp-json/wp/v2` exposes posts,
//!   categories, search, and pagination server-side. The 27-post
//!   `Workout of the Week` category (id 15) holds real pool workouts;
//!   `Whiteboard Wednesday` (id 14) is technique video posts (no sets).
//! - Each workout article is `Warmup / Pre-Set (Nx) / Main Set (Nx) /
//!   Cool Down` headings with `<li>` steps (`1×300 Freestyle @ 5:00`,
//!   `4×50 Kick @ 1:10`), normalized to the same notation text the
//!   generator parses (see [`normalize_body`]).
//! - Normalization is lossy by design: dryland cross-training steps
//!   (`25 Butterfly + 5 Pushups`) and open-water narrative posts have no
//!   swim distance and are dropped — the parser only swims digits.
//! - Scrape politely: browser UA, 1 req/s (shared [`PAGE_DELAY`]).

use super::swimdojo::PAGE_DELAY;
use super::traits::{ListFilter, ListingItem, ScrapedWorkout, Site, SiteError};

/// WordPress API root for the MySwimPro blog.
pub const API_BASE: &str = "https://blog.myswimpro.com/wp-json/wp/v2";

/// Category holding real pool workouts (27 posts, verified 2026-09-09).
pub const WORKOUT_CATEGORY: u32 = 15;

/// A myswimpro.com workout site.
#[derive(Debug, Clone)]
pub struct Myswimpro {
    agent: ureq::Agent,
}

impl Default for Myswimpro {
    fn default() -> Self {
        Self::new()
    }
}

impl Myswimpro {
    /// Build a scraper with the default agent (browser UA, 10 s timeout).
    pub fn new() -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .user_agent(super::swimdojo::USER_AGENT)
            .timeout_global(Some(std::time::Duration::from_secs(10)))
            .build()
            .into();
        Self { agent }
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

    /// Parse a `/posts` response into items; pool workouts only.
    pub fn parse_posts(json: &str) -> Result<Vec<ListingItem>, SiteError> {
        Ok(Self::parse_posts_with(json)?
            .into_iter()
            .map(|p| p.item)
            .collect())
    }

    /// Parse a `/posts` response into items plus their body HTML (for
    /// `fetch`, which normalizes `content.rendered` directly).
    fn parse_posts_with(json: &str) -> Result<Vec<PostWithBody>, SiteError> {
        let posts: Vec<WpPost> =
            serde_json::from_str(json).map_err(|e| SiteError(format!("parsing posts: {e}")))?;
        Ok(posts
            .iter()
            .map(|p| PostWithBody {
                title: html_escape::decode_html_entities(&p.title.rendered)
                    .trim()
                    .to_owned(),
                body_html: p
                    .content
                    .as_ref()
                    .map(|c| c.rendered.clone())
                    .unwrap_or_default(),
                item: ListingItem::from(p),
            })
            .collect())
    }
}

impl Site for Myswimpro {
    fn list(&self, filter: &ListFilter) -> Result<Vec<ListingItem>, SiteError> {
        let want = filter.limit.unwrap_or(usize::MAX).min(100);
        let mut url = format!(
            "{API_BASE}/posts?categories={WORKOUT_CATEGORY}&per_page={want}&_fields=link,title,date,content,categories"
        );
        if let Some(q) = &filter.query {
            url.push_str(&format!("&search={q}"));
        }
        let json = self.get(&url)?;
        let mut items = Self::parse_posts(&json)?;
        if filter.query.is_some() {
            // `search=` matches whole-post text server-side; keep only
            // title/summary hits like the swimdojo query filter.
            items.retain(|i| {
                filter
                    .query
                    .as_deref()
                    .is_some_and(|q| super::swimdojo::matches_query(i, q))
            });
        }
        if let Some(n) = filter.limit {
            items.truncate(n);
        }
        std::thread::sleep(PAGE_DELAY);
        Ok(items)
    }

    fn fetch(&self, url: &str) -> Result<ScrapedWorkout, SiteError> {
        // Prefer the WP API: `?p=<id>`-free lookup by slug returns
        // `content.rendered` without theme/chrome markup.
        if let Some(slug) = url.trim_end_matches('/').rsplit('/').next()
            && !slug.is_empty()
        {
            let api = format!("{API_BASE}/posts?slug={slug}&_fields=link,title,content");
            if let Ok(json) = self.get(&api)
                && let Ok(mut posts) = Self::parse_posts_with(&json)
                && let Some(post) = posts.pop()
            {
                let title = post.title.clone();
                let body = normalize_body(&post.body_html);
                if !body.trim().is_empty() {
                    return Ok(ScrapedWorkout {
                        title,
                        url: url.to_owned(),
                        body,
                    });
                }
            }
        }
        // Fallback: raw HTML page (Squarespace-style content div or whole
        // page — the normalizer windows on section headings itself).
        let page = self.get(url)?;
        let title = page_title(&page);
        let body = normalize_body(&page);
        if body.trim().is_empty() {
            return Err(SiteError(format!("no workout body in {url}")));
        }
        Ok(ScrapedWorkout {
            title,
            url: url.to_owned(),
            body,
        })
    }
}

#[derive(Debug, serde::Deserialize)]
struct WpPost {
    link: String,
    title: WpRendered,
    #[serde(default)]
    content: Option<WpRendered>,
    #[serde(default)]
    date: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct WpRendered {
    rendered: String,
}

struct PostWithBody {
    title: String,
    body_html: String,
    item: ListingItem,
}

impl From<&WpPost> for ListingItem {
    fn from(post: &WpPost) -> Self {
        // WP `date` is local wall time (`2022-06-09T12:40:42`, no offset),
        // not RFC 3339 — parse it naively as UTC.
        let published = post.date.as_deref().and_then(|d| {
            chrono::NaiveDateTime::parse_from_str(d, "%Y-%m-%dT%H:%M:%S")
                .ok()
                .map(|n| n.and_utc().timestamp())
        });
        let description = post
            .content
            .as_ref()
            .map(|c| text_of(&c.rendered))
            .map(|t| t.chars().take(200).collect());
        Self {
            title: html_escape::decode_html_entities(&post.title.rendered)
                .trim()
                .to_owned(),
            url: post.link.clone(),
            author: None,
            tags: vec!["myswimpro".to_owned()],
            description,
            published,
        }
    }
}

fn page_title(page: &str) -> String {
    super::swimdojo::detail_body(page)
        .map(|(t, _)| t)
        .unwrap_or_default()
}

/// Normalize a workout article (WP `content.rendered` or full HTML page)
/// into parser-ready notation text.
///
/// Starts at the first section heading (`Warmup`, `Pre-Set`, `Main Set`,
/// `Cool Down`, `Kick & Drill`, `IM Set`, `Scull Set`, … — see
/// [`is_section_header`]) and keeps digit-led `<li>`/`<p>` steps plus
/// `Round N:` equipment notes (as `—>` annotations); drops meta items
/// (`Distance:`, `Duration:`, `Equipment:`), related-link paragraphs,
/// dryland steps (no swim distance), and everything outside the
/// first-heading … last-step window. Unknown `X Set` headings are kept
/// verbatim as sub-section markers — the parser attaches them as notes
/// to the previous step, so no workout text is silently lost.
pub fn normalize_body(html: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut started = false;
    for block in super::swimdojo::blocks(html) {
        let text = super::swimdojo::clean_block(&block);
        if text.is_empty() {
            continue;
        }
        if !started {
            if is_section_header(&text) {
                started = true;
                out.push(header_of(&text));
            }
            continue;
        }
        if is_section_header(&text) {
            out.push(header_of(&text));
        } else if is_meta(&text) || is_related(&text) {
            continue;
        } else if text
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit() || c == '×')
        {
            let step = normalize_step(&text);
            if !step.is_empty() {
                out.push(step);
            }
        } else if text.starts_with("Round ") || text.starts_with("*Round ") {
            out.push(format!("—>{}", text.trim_start_matches('*')));
        }
        // Other prose (intro, tips, comments) is dropped like the
        // swimdojo normalizer drops non-step prose without a step.
    }
    // Trim trailing non-step lines (share prompts, next-article teasers).
    while out.last().is_some_and(|l| !is_step_line(l)) {
        out.pop();
    }
    let mut body = out.join("\n");
    if !body.is_empty() {
        body.push('\n');
    }
    body
}

/// A section heading: a known label (`Warmup`, `Pre-Set`, `Main Set`,
/// `Cool Down`, …) or a drill/set heading (`Kick & Drill`, `IM Set`,
/// `Scull Set`, `Scull Set (2 Rounds)`) — i.e. a short non-step line
/// ending in `set`/`drill` or carrying a repeat suffix (`Main Set (3x)`,
/// `IM Set (x2)`).
fn is_section_header(text: &str) -> bool {
    if repeat_suffix(text).is_some() {
        return true;
    }
    let low = text.to_ascii_lowercase();
    let bare = low
        .split(['(', ':'])
        .next()
        .unwrap_or(&low)
        .trim()
        .trim_end_matches('s');
    if matches!(
        bare,
        "warmup"
            | "warm-up"
            | "warm up"
            | "pre-set"
            | "pre set"
            | "main set"
            | "mainset"
            | "main"
            | "cool down"
            | "cooldown"
            | "cool-down"
            | "post-main set"
    ) {
        return true;
    }
    // Drill/set headings: short, no digits, ends in set/drill/section —
    // or a `The Workout (…)` header with a parenthetical (tethered
    // time-based workout). A bare `The Workout` h4 is an article
    // subheading, not a section — skip it (the real headings follow).
    if low.starts_with("the workout") {
        return text.contains('(');
    }
    !text.chars().any(|c| c.is_ascii_digit())
        && text.len() < 30
        && ["set", "drill", "section"]
            .iter()
            .any(|w| bare.ends_with(w))
}

fn header_of(text: &str) -> String {
    // `The Workout (30 seconds rest …)` is a verbatim header: the
    // parenthetical is prose, not a repeat count.
    if text.to_ascii_lowercase().starts_with("the workout") {
        return text.to_owned();
    }
    let low = text.to_ascii_lowercase();
    let bare = low.split(['(', ':']).next().unwrap_or(&low).trim();
    let count = repeat_suffix(text);
    let label = if bare.contains("warm") {
        "Warm Up".to_owned()
    } else if bare.contains("pre") {
        "Pre-Set".to_owned()
    } else if bare.contains("post") {
        "Post-Main".to_owned()
    } else if bare.contains("main") {
        "Main Set".to_owned()
    } else if bare.contains("cool") || bare.contains("down") {
        "Warm Down".to_owned()
    } else {
        text.split('(').next().unwrap_or(text).trim().to_owned()
    };
    match count {
        Some(n) => format!("{label} ({n}x)"),
        None => label,
    }
}

/// `(2x)` / `(3x)` / `(x2)` suffix on a heading.
fn repeat_suffix(text: &str) -> Option<u32> {
    let inner = text.split('(').nth(1)?.split(')').next()?;
    let inner = inner.trim().trim_start_matches('*');
    inner
        .strip_suffix(['x', 'X'])
        .and_then(|n| n.trim().parse().ok())
        .or_else(|| {
            inner
                .strip_prefix(['x', 'X'])
                .and_then(|n| n.trim().parse().ok())
        })
}

fn is_meta(text: &str) -> bool {
    let low = text.to_ascii_lowercase();
    ["distance:", "duration:", "equipment:"]
        .iter()
        .any(|m| low.starts_with(m))
}

fn is_related(text: &str) -> bool {
    let low = text.to_ascii_lowercase();
    low.starts_with("related:") || low.starts_with("like this workout")
}

fn is_step_line(line: &str) -> bool {
    line.chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit() || c == '×')
        || line.starts_with("—>")
}

/// `4×50 Kick @ 1:10` → `4 x 50 kick @ 1:10`; `1×200 Freestyle` → keep.
/// Drops dryland steps (`+ 5 Pushups`) and `50s`-style tokens pass
/// through for the parser's drill/unit handling.
pub fn normalize_step(text: &str) -> String {
    if text.contains('+') && !text.contains('@') {
        return String::new();
    }
    if text.split_whitespace().any(|w| {
        matches!(
            w.to_ascii_lowercase().as_str(),
            "pushups" | "squats" | "twists" | "raises"
        )
    }) {
        return String::new();
    }
    let mut s: String = text
        .chars()
        .flat_map(|c| {
            if c == '×' {
                " x ".chars().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect();
    s = s.replace('\u{a0}', " ");
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn text_of(html: &str) -> String {
    html_escape::decode_html_entities(&super::swimdojo::strip_tags(html))
        .replace('\u{a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
#[path = "myswimpro_tests.rs"]
mod myswimpro_tests;
