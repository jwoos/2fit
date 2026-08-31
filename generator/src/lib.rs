//! `fit_generator` — turns written workouts into .fit files.
//!
//! - `parser`: format-specific text parsers → `fit_core` (first: swimdojo)
//! - `fit`: `fit_core` → .fit bytes via `rustyfit`

pub mod fit;
pub mod parser;
