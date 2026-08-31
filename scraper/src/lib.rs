//! `fit_scraper` — scrapes written workouts from swim sites and emits
//! normalized text that `fit_generator` can parse.
//!
//! - `site`: one module per target site behind a shared trait (first: swimdojo)
//! - `schema`: describes a site's workout format as data the generator consumes

pub mod schema;
pub mod site;
