//! `2fit-scrape`: list and fetch swim workouts as normalized text.

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use fit_scraper::site::{ListFilter, Myswimpro, Site, Swimdojo};

/// Which workout site to scrape.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum SiteKind {
    /// swimdojo.com (Squarespace RSS feed).
    Swimdojo,
    /// myswimpro.com blog (WordPress API, Workout-of-the-Week).
    Myswimpro,
}

/// Scrape swim workouts (swimdojo.com or myswimpro.com blog).
#[derive(Debug, Parser)]
#[command(name = "2fit-scrape", version)]
struct Args {
    /// Which site to scrape.
    #[arg(long, value_enum, default_value = "swimdojo")]
    site: SiteKind,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// List workouts (title + URL + summary).
    List {
        /// Match the site's tag label exactly (swimdojo e.g. `IM`).
        #[arg(long)]
        tag: Option<String>,
        /// Match the site's author id (swimdojo e.g. `5aa560f75ce350fbdd62294b`).
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
        /// Workout page URL (or site-local slug from `list`).
        url: String,
    },
    /// Infer a workout-format schema from sample notation files.
    Schema {
        /// Sample notation files (one workout body each).
        files: Vec<std::path::PathBuf>,
        /// Schema name (default `custom`).
        #[arg(long, default_value = "custom")]
        name: String,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.cmd {
        Cmd::List {
            tag,
            author,
            query,
            limit,
        } => {
            let filter = ListFilter {
                tag,
                author,
                query,
                limit,
            };
            let items = match args.site {
                SiteKind::Swimdojo => Swimdojo::new().list(&filter)?,
                SiteKind::Myswimpro => Myswimpro::new().list(&filter)?,
            };
            for i in items {
                println!("{}\n  {}\n", i.title, i.url);
            }
        }
        Cmd::Fetch { url } => {
            let w = match args.site {
                SiteKind::Swimdojo => Swimdojo::new().fetch(&normalize_swimdojo(&url))?,
                SiteKind::Myswimpro => Myswimpro::new().fetch(&url)?,
            };
            print!("{}", w.body);
        }
        Cmd::Schema { files, name } => {
            let samples = files
                .iter()
                .map(|path| {
                    let body = std::fs::read_to_string(path)
                        .map_err(|e| anyhow::anyhow!("reading {}: {e}", path.display()))?;
                    Ok(fit_scraper::site::ScrapedWorkout {
                        title: path.display().to_string(),
                        url: String::new(),
                        body,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let inf = fit_scraper::schema::infer(&name, &samples);
            println!("{}", serde_json::to_string_pretty(&inf.schema)?);
            if !inf.gaps.is_empty() || !inf.unclassified.is_empty() {
                eprintln!("{inf}");
            }
        }
    }
    Ok(())
}

/// Accept a bare slug (or `/workouts/…` path) as shorthand for the full URL.
fn normalize_swimdojo(url: &str) -> String {
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
            normalize_swimdojo("https://www.swimdojo.com/workouts/2021/2/16/box-crab"),
            "https://www.swimdojo.com/workouts/2021/2/16/box-crab"
        );
        assert_eq!(
            normalize_swimdojo("2021/2/16/box-crab"),
            "https://www.swimdojo.com/workouts/2021/2/16/box-crab"
        );
        assert_eq!(
            normalize_swimdojo("/workouts/2021/2/16/box-crab"),
            "https://www.swimdojo.com/workouts/2021/2/16/box-crab"
        );
    }
}
