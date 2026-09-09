//! `2fit-scrape`: list and fetch swimdojo workouts as normalized text.

use anyhow::Result;
use clap::{Parser, Subcommand};
use fit_scraper::site::{ListFilter, Site, Swimdojo};

/// Scrape swim workouts from swimdojo.com (via its RSS feed).
#[derive(Debug, Parser)]
#[command(name = "2fit-scrape", version)]
struct Args {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// List workouts (title + URL + summary).
    List {
        /// Match the site's tag label exactly (e.g. `IM`, `Triathlon`).
        #[arg(long)]
        tag: Option<String>,
        /// Match the site's author id (e.g. `5aa560f75ce350fbdd62294b`).
        #[arg(long)]
        author: Option<String>,
        /// Free-text match against title/description/tags.
        #[arg(long)]
        query: Option<String>,
        /// Maximum workouts to list.
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Fetch one workout page as normalized notation text.
    Fetch {
        /// Workout page URL (or slug from `list`).
        url: String,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();
    let site = Swimdojo::new();
    match args.cmd {
        Cmd::List {
            tag,
            author,
            query,
            limit,
        } => {
            let items = site.list(&ListFilter {
                tag,
                author,
                query,
                limit,
            })?;
            for i in items {
                println!("{}\n  {}\n", i.title, i.url);
            }
        }
        Cmd::Fetch { url } => {
            let url = normalize_url(&url);
            let w = site.fetch(&url)?;
            print!("{}", w.body);
        }
    }
    Ok(())
}

/// Accept a bare slug (or `/workouts/…` path) as shorthand for the full URL.
fn normalize_url(url: &str) -> String {
    if url.starts_with("http") {
        return url.to_owned();
    }
    let path = url.trim_start_matches('/');
    if path.starts_with("workouts/") {
        return format!("https://www.swimdojo.com/{path}");
    }
    format!("https://www.swimdojo.com/workouts/{path}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_shorthands() {
        assert_eq!(
            normalize_url("https://www.swimdojo.com/workouts/2021/2/16/box-crab"),
            "https://www.swimdojo.com/workouts/2021/2/16/box-crab"
        );
        assert_eq!(
            normalize_url("2021/2/16/box-crab"),
            "https://www.swimdojo.com/workouts/2021/2/16/box-crab"
        );
        assert_eq!(
            normalize_url("/workouts/2021/2/16/box-crab"),
            "https://www.swimdojo.com/workouts/2021/2/16/box-crab"
        );
    }
}
