//! One module per target site behind a shared trait (swimdojo, myswimpro).

pub mod myswimpro;
pub mod swimdojo;
pub mod traits;

pub use myswimpro::Myswimpro;
pub use swimdojo::Swimdojo;
pub use traits::{ListFilter, ListingItem, ScrapedWorkout, Site, SiteError};
