//! The shared interface every scraped site implements.
//!
//! A site lists workout summaries (title, link, tags) and renders each
//! workout as normalized notation text — the same plain-text form stored in
//! `documents/swimdojo-fixtures/*.txt` that `fit_generator` parses.

use std::fmt;

/// One workout entry from a site's listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingItem {
    /// Workout title, e.g. `Goblin Shark`.
    pub title: String,
    /// Absolute URL of the workout's page.
    pub url: String,
    /// Author name, if the listing exposes one.
    pub author: Option<String>,
    /// Tag/facet labels, if the listing exposes any.
    pub tags: Vec<String>,
    /// Blurb/summary, if the listing exposes one.
    pub description: Option<String>,
    /// Publication timestamp (seconds since epoch), if known.
    pub published: Option<i64>,
}

/// A workout page rendered as normalized notation text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrapedWorkout {
    /// Workout title.
    pub title: String,
    /// Absolute URL of the workout's page.
    pub url: String,
    /// The body in normalized notation (parseable by `fit_generator`).
    pub body: String,
}

/// Filters accepted when listing a site's workouts. Every filter is
/// optional; sites ignore what they cannot express server-side (documented
/// per site) and callers refine client-side from [`ListingItem`] fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListFilter {
    /// Match the site's tag/category label (e.g. `IM`).
    pub tag: Option<String>,
    /// Match the site's author identifier or name.
    pub author: Option<String>,
    /// Maximum workouts to return (client-side truncation).
    pub limit: Option<usize>,
    /// Free-text match against title/description/tags (client-side).
    pub query: Option<String>,
}

/// Failure to list or fetch a workout page.
#[derive(Debug)]
pub struct SiteError(pub String);

impl fmt::Display for SiteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SiteError {}

/// A workout site: list summaries, then fetch each body as notation text.
pub trait Site {
    /// List workout summaries matching `filter`, oldest cursor first.
    fn list(&self, filter: &ListFilter) -> Result<Vec<ListingItem>, SiteError>;

    /// Fetch one workout page and render its body as notation text.
    fn fetch(&self, url: &str) -> Result<ScrapedWorkout, SiteError>;
}
