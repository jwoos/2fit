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

/// Parse notation driven by a stored [`fit_core::FormatSchema`].
///
/// The schema's vocabulary tables replace swimdojo's words; structural
/// rules are shared. Used with schemas from `fit_scraper::schema::infer`
/// (or hand-written JSON via [`parser::schema::swimdojo`] as a template).
pub fn parse_with_schema(
    text: &str,
    pool: fit_core::Pool,
    base100: Option<fit_core::Seconds>,
    schema: &fit_core::FormatSchema,
) -> Result<fit_core::Workout, parser::swimdojo::Error> {
    parser::swimdojo::parse_with_schema(text, pool, base100, schema)
}

/// Parse notation with athlete thresholds (`--run-base`/`--ftp` values).
///
/// Sugar over [`parser::swimdojo::parse_with_thresholds`]: resolves
/// `%FTP`/`% of pace` targets during parse; without thresholds those stay
/// notes.
pub fn parse_with_thresholds(
    text: &str,
    pool: fit_core::Pool,
    base100: Option<fit_core::Seconds>,
    thresholds: parser::swimdojo::Thresholds,
    schema: &fit_core::FormatSchema,
) -> Result<fit_core::Workout, parser::swimdojo::Error> {
    parser::swimdojo::parse_with_thresholds(text, pool, base100, thresholds, schema)
}
