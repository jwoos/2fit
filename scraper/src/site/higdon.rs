//! Hal Higdon training programs (halhigdon.com), via plain-HTML plan pages.
//!
//! Why plain-HTML table scraping (evidence, 2026-09-21):
//!
//! - Each program page is a static grid: two `table.tablesaw` tables (miles
//!   first, km second — same cells, e.g. `3 mi run` / `4.8 km run`), 18
//!   rows + header (`Week | Mon..Sun`). Verified on Novice 1 Marathon
//!   (saved `/tmp/higdon.html`, 118951 B, 2 tables × 19 rows).
//! - Cells are one workout line each: `<N> mi|km run`, bare long-run
//!   distances (`6`, `20` — unit from the table: miles or km), `Rest`,
//!   `Cross`, race names (`Half Marathon`, `Marathon`, `5K Race`).
//! - Program links are static `training-programs/<group>/<slug>/` hrefs
//!   (14 on Novice 1: marathon groups + cross-links); `list` filters them
//!   by slug substring client-side — no server search exists.
//! - Scrape politely: browser UA, 1 req/s (shared [`PAGE_DELAY`]).
//!
//! Normalization maps one plan week to one notation body: `Week N` header,
//! then one cell per day (order preserved top-to-bottom; day prefixes are
//! dropped — the parser has no day model). Day markers (`Rest`, `Cross`,
//! race names) become `—>` annotations on the week — the IDL has no
//! rest-day step (`run-bike-notation.md` §1: they map to "no step", not
//! `Step::Rest`).

use super::swimdojo::{PAGE_DELAY, USER_AGENT, strip_tags};
use super::traits::{ListFilter, ListingItem, ScrapedWorkout, Site, SiteError};

/// Host holding all Higdon programs.
pub const SITE_BASE: &str = "https://www.halhigdon.com";

/// Novice 1 Marathon grid (evidence page; every plan shares the shape).
pub const NOVICE1_URL: &str =
    "https://www.halhigdon.com/training-programs/marathon-training/novice-1-marathon/";

/// One plan week: `(week_no, [Mon..Sun cells])`.
pub type PlanWeek = (String, Vec<String>);

/// One grid table: `(unit, weeks)` (`"mi"` miles or `"km"` km view).
pub type PlanTable = (String, Vec<PlanWeek>);

/// A halhigdon.com training-program site.
#[derive(Debug, Clone)]
pub struct Higdon {
    agent: ureq::Agent,
}

impl Default for Higdon {
    fn default() -> Self {
        Self::new()
    }
}

impl Higdon {
    /// Build a scraper with the default agent (browser UA, 10 s timeout).
    pub fn new() -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .user_agent(USER_AGENT)
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

    /// Program links (`training-programs/<group>/<slug>/`) on a page.
    /// Returns `(title, url)` with titles from slug title-case
    /// (`novice-1-marathon` → `Novice 1 Marathon`); the plan page title
    /// comes from `fetch`.
    pub fn parse_program_links(html: &str) -> Vec<(String, String)> {
        let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        // Raw-href scan: `href="…"` pairs.
        let mut rest = html;
        while let Some(i) = rest.find("href=\"") {
            rest = &rest[i + 6..];
            let Some(end) = rest.find('"') else { break };
            let href = &rest[..end];
            rest = &rest[end + 1..];
            if !href.contains("training-programs/") {
                continue;
            }
            let url = if href.starts_with("http") {
                href.to_owned()
            } else {
                format!("{SITE_BASE}{href}")
            };
            seen.insert(url);
        }
        Self::parse_program_links_sorted(seen)
    }

    fn parse_program_links_sorted(
        seen: std::collections::BTreeSet<String>,
    ) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = seen
            .into_iter()
            .map(|url| {
                let slug = url
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or("")
                    .to_owned();
                let title = slug
                    .split('-')
                    .map(|w| {
                        let mut c = w.chars();
                        match c.next() {
                            Some(f) => f.to_ascii_uppercase().to_string() + c.as_str(),
                            None => String::new(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                (slug, title, url)
            })
            .map(|(slug, title, url)| (format!("{title} ({slug})"), url))
            .collect();
        out.sort();
        out
    }

    /// All grid tables on a plan page: `(unit, weeks)` where each week is
    /// `(week_no, [Mon..Sun cells])`. `unit` is `"mi"` (first table) or
    /// `"km"` (second).
    pub fn parse_plan_tables(html: &str) -> Vec<PlanTable> {
        let tables = table_htmls(html);
        let mut out = Vec::new();
        for (i, t) in tables.iter().enumerate() {
            let rows = table_rows(t);
            if rows.is_empty() || !rows[0].iter().any(|c| c.eq_ignore_ascii_case("mon")) {
                continue;
            }
            let unit = if i == 0 { "mi" } else { "km" }.to_owned();
            let mut weeks = Vec::new();
            for row in rows.into_iter().skip(1) {
                if row.is_empty() {
                    continue;
                }
                let week_no = row.first().cloned().unwrap_or_default();
                let mut cells: Vec<String> = row.into_iter().skip(1).collect();
                while cells.len() < 7 {
                    cells.push(String::new());
                }
                cells.truncate(7);
                weeks.push((week_no, cells));
            }
            out.push((unit, weeks));
        }
        out
    }

    /// Page `<title>` → plan title (`Novice 1 Marathon Training Program`).
    pub fn page_title(html: &str) -> String {
        let start = html.find("<title>").map(|i| i + 7);
        let title = start
            .and_then(|s| {
                html[s..]
                    .find("</title>")
                    .map(|e| html[s..s + e].to_owned())
            })
            .unwrap_or_default();
        html_escape::decode_html_entities(&strip_tags(&title))
            .split('|')
            .next()
            .unwrap_or("")
            .trim()
            .to_owned()
    }
}

impl Site for Higdon {
    fn list(&self, filter: &ListFilter) -> Result<Vec<ListingItem>, SiteError> {
        // No listing endpoint exists: seed from the evidence page's links.
        let html = self.get(NOVICE1_URL)?;
        let mut items: Vec<ListingItem> = Self::parse_program_links(&html)
            .into_iter()
            .map(|(title, url)| ListingItem {
                title,
                url,
                author: Some("Hal Higdon".to_owned()),
                tags: vec!["higdon".to_owned(), "run".to_owned()],
                description: None,
                published: None,
            })
            .collect();
        if let Some(q) = &filter.query
            && !q.is_empty()
        {
            items.retain(|i| super::swimdojo::matches_query(i, q));
        }
        if let Some(tag) = &filter.tag
            && !tag.is_empty()
        {
            let t = tag.to_ascii_lowercase();
            items.retain(|i| {
                i.title.to_ascii_lowercase().contains(&t) || i.url.to_ascii_lowercase().contains(&t)
            });
        }
        if let Some(n) = filter.limit {
            items.truncate(n);
        }
        std::thread::sleep(PAGE_DELAY);
        Ok(items)
    }

    fn fetch(&self, url: &str) -> Result<ScrapedWorkout, SiteError> {
        let html = self.get(url)?;
        let tables = Self::parse_plan_tables(&html);
        if tables.is_empty() {
            return Err(SiteError(format!("no training grid in {url}")));
        }
        let title = Self::page_title(&html);
        let title = if title.is_empty() {
            url.to_owned()
        } else {
            title
        };
        let body = normalize_weeks(&tables);
        if body.trim().is_empty() {
            return Err(SiteError(format!("empty training grid in {url}")));
        }
        Ok(ScrapedWorkout {
            title,
            url: url.to_owned(),
            body,
        })
    }
}

/// One plan week → notation lines: `Week N` + `Mon: <cell>` … + `—>Rest
/// days: …` for day markers. `tables` holds both unit views; the miles
/// table wins (imperial source; km cells are conversions of it).
pub fn normalize_weeks(tables: &[PlanTable]) -> String {
    const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let mi = tables.iter().find(|(u, _)| u == "mi").or(tables.first());
    let Some((_, weeks)) = mi else {
        return String::new();
    };
    let mut out = Vec::new();
    for (week_no, cells) in weeks {
        out.push(format!("Week {week_no}"));
        let mut markers: Vec<String> = Vec::new();
        for (day, cell) in DAYS.iter().zip(cells.iter()) {
            let cell = normalize_cell(cell);
            if is_day_marker(&cell) {
                markers.push(format!("{day} {cell}"));
            } else if !cell.is_empty() {
                // Day prefixes dropped: the parser has no day model (a
                // `Tue:` prefix would become note text on the step).
                out.push(cell);
            }
        }
        if !markers.is_empty() {
            out.push(format!("—>Rest days: {}", markers.join(", ")));
        }
    }
    let mut body = out.join("\n");
    if !body.is_empty() {
        body.push('\n');
    }
    body
}

/// A grid cell → notation: `3 mi run` verbatim; bare `6` → `6 mi` (miles
/// table; the km table's `9.7` → `9.7 km` handled by the caller choosing
/// the miles view); whitespace collapsed.
pub fn normalize_cell(cell: &str) -> String {
    let cell = cell.split_whitespace().collect::<Vec<_>>().join(" ");
    if cell.is_empty() {
        return cell;
    }
    if cell.chars().all(|c| c.is_ascii_digit()) {
        return format!("{cell} mi");
    }
    cell
}

/// Whole-day markers (no IDL step): `Rest`, `Cross`, race names.
/// Evidence: Novice 1 grid cells (`Rest`, `Cross`, `Half Marathon`,
/// `Marathon`); 5K/10K grids add `5K Test`, `5K Race`, `10K Race`.
pub fn is_day_marker(cell: &str) -> bool {
    let low = cell.to_ascii_lowercase();
    low == "rest"
        || low == "cross"
        || low.ends_with("marathon")
        || low.ends_with("race")
        || low.ends_with("test")
}

/// Raw `<table>…</table>` inner HTMLs in document order.
fn table_htmls(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(i) = rest.find("<table") {
        let after = match rest[i..].find('>') {
            Some(j) => i + j + 1,
            None => break,
        };
        let Some(end) = rest[after..].find("</table>") else {
            break;
        };
        out.push(rest[after..after + end].to_owned());
        rest = &rest[after + end + 8..];
    }
    out
}

/// Rows of one table as cleaned cell texts.
fn table_rows(table: &str) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    let mut rest = table;
    while let Some(i) = rest.find("<tr") {
        let after = match rest[i..].find('>') {
            Some(j) => i + j + 1,
            None => break,
        };
        let Some(end) = rest[after..].find("</tr>") else {
            break;
        };
        let row = &rest[after..after + end];
        rest = &rest[after + end + 5..];
        let mut cells = Vec::new();
        let mut c = row;
        loop {
            let th = c.find("<th");
            let td = c.find("<td");
            let pos = match (th, td) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            };
            let Some(p) = pos else { break };
            let inner = match c[p..].find('>') {
                Some(j) => p + j + 1,
                None => break,
            };
            let close_th = c[inner..].find("</th>");
            let close_td = c[inner..].find("</td>");
            let end = match (close_th, close_td) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            };
            let Some(e) = end else { break };
            let text = html_escape::decode_html_entities(&strip_tags(&c[inner..inner + e]))
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            cells.push(text);
            c = &c[inner + e + 5..];
        }
        out.push(cells);
    }
    out
}

#[cfg(test)]
#[path = "higdon_tests.rs"]
mod higdon_tests;
