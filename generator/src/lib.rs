//! `fit_generator` — turns written workouts into .fit files.
//!
//! - `parser`: format-specific text parsers → `fit_core` (first: swimdojo)
//! - `fit`: `fit_core` → .fit bytes via `rustyfit`

pub mod fit;
pub mod parser;

pub use fit_core;

/// Parse swimdojo workout notation into a [`fit_core::Workout`].
///
/// Sugar over [`parser::swimdojo::parse`]; `pool` supplies the distance
/// unit, `base100` the swimmer's base per 100 (required iff the text uses
/// `@ b` intervals — resolution is deferred, so `None` parses fine and
/// only matters when resolving).
pub fn parse_str(
    text: &str,
    pool: fit_core::Pool,
    base100: Option<fit_core::Seconds>,
) -> Result<fit_core::Workout, parser::swimdojo::Error> {
    parser::swimdojo::parse(text, pool, base100)
}
