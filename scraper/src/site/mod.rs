//! One module per target site behind a shared trait (first: swimdojo).

pub mod swimdojo;
pub mod traits;

pub use swimdojo::Swimdojo;
pub use traits::{ListFilter, ListingItem, ScrapedWorkout, Site, SiteError};
