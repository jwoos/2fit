//! Stored workout-format schemas: vocabulary-as-data for the parser.
//!
//! A schema is a [`fit_core::FormatSchema`] value serialized as JSON. The
//! parser reads its tables from a schema instead of literals, so a new
//! site's format is a new JSON file — not a new parser. [`swimdojo()`]
//! builds the swimdojo notation schema (vocabulary evidenced in
//! `documents/swimdojo-grammar.txt`, wired through the parser in turn 9).

use fit_core::{FormatSchema, SectionLabel, Stroke};

/// The swimdojo.com notation schema.
///
/// Every table entry traces to the parser behavior it preserves:
/// section labels (`Warm Up`, `Main`/`Main Set`, `Set N`, `Warm Down` /
/// `Cool Down`), strokes (`back`/`backstroke`, `breast`/`breaststroke`,
/// `fly`/`butterfly`, `im`/`imedley`, lone `stroke` ⇒ `Any`), silent
/// freestyle words, rep-counted drills (`bobs`, `sculls`), the `@`
/// interval marker with `kb` (clock, kept as note) and `b` (base) markers,
/// rest words (`30 seconds rest`), the `easy` recovery word, and the
/// `through` / `—>` / ` x ` structural markers.
pub fn swimdojo() -> FormatSchema {
    FormatSchema {
        name: "swimdojo".to_owned(),
        section_labels: vec![
            ("warm up".to_owned(), SectionLabel::WarmUp),
            ("warmup".to_owned(), SectionLabel::WarmUp),
            ("warm down".to_owned(), SectionLabel::CoolDown),
            ("cool down".to_owned(), SectionLabel::CoolDown),
            ("cooldown".to_owned(), SectionLabel::CoolDown),
            ("main set".to_owned(), SectionLabel::Main),
            ("main".to_owned(), SectionLabel::Main),
        ],
        total_prefix: "total".to_owned(),
        repeat_word: "through".to_owned(),
        annotation_markers: vec![
            "—>".to_owned(),
            "–>".to_owned(),
            "-->".to_owned(),
            "--".to_owned(),
            "—".to_owned(),
            "–".to_owned(),
        ],
        strokes: vec![
            ("back".to_owned(), Stroke::Back),
            ("backstroke".to_owned(), Stroke::Back),
            ("breast".to_owned(), Stroke::Breast),
            ("breaststroke".to_owned(), Stroke::Breast),
            ("fly".to_owned(), Stroke::Fly),
            ("butterfly".to_owned(), Stroke::Fly),
            ("im".to_owned(), Stroke::IM),
            ("imedley".to_owned(), Stroke::IM),
            ("stroke".to_owned(), Stroke::Any),
        ],
        freestyle_words: vec!["free".to_owned(), "swim".to_owned()],
        drill_words: vec![
            "bob".to_owned(),
            "bobs".to_owned(),
            "scull".to_owned(),
            "sculls".to_owned(),
        ],
        interval_marker: "@".to_owned(),
        clock_prefix: "kb".to_owned(),
        base_marker: "b".to_owned(),
        rest_words: vec!["rest".to_owned(), "second".to_owned(), "seconds".to_owned()],
        recovery_word: "easy".to_owned(),
        count_separator: " x ".to_owned(),
    }
}

/// The Hal Higdon run notation schema.
///
/// Divergences from swimdojo, evidenced by the Intermediate 5K/10K plans
/// (see `documents/run-fixtures/` + fetched grids in `run-bike-notation.md`
/// §1): cells are `<N> mi|km run`, `<N> min tempo|fast|cross`, `N x 400`
/// track reps with `5K pace` qualifiers, `Rest`/`Cross`/race day markers.
/// Structural rules are shared; only vocabulary differs. Run distances
/// carry explicit units (`suffixed_distance` beats the pool) and durations
/// parse via `leading_duration`; pace targets stay notes in Phase 3
/// (`run_base` resolution deferred — no new schema fields needed).
pub fn higdon_run() -> FormatSchema {
    let base = swimdojo();
    FormatSchema {
        name: "higdon-run".to_owned(),
        section_labels: vec![
            ("warm up".to_owned(), SectionLabel::WarmUp),
            ("warmup".to_owned(), SectionLabel::WarmUp),
            ("main set".to_owned(), SectionLabel::Main),
            ("main".to_owned(), SectionLabel::Main),
            ("cool down".to_owned(), SectionLabel::CoolDown),
            ("cooldown".to_owned(), SectionLabel::CoolDown),
            ("warm down".to_owned(), SectionLabel::CoolDown),
        ],
        freestyle_words: vec!["run".to_owned(), "runs".to_owned(), "fast".to_owned()],
        drill_words: vec![],
        rest_words: vec![
            "rest".to_owned(),
            "cross".to_owned(),
            "x-train".to_owned(),
            "xtrain".to_owned(),
        ],
        recovery_word: "tempo".to_owned(),
        ..base
    }
}

/// The Zwift-style bike/run interval schema.
///
/// Evidenced by whatsonzwift.com excerpts (`documents/bike-fixtures/` +
/// `documents/run-fixtures/zwift-101-w1-best-mile.txt`): spaceless durations
/// (`5min free ride`, `1min @ 85rpm, 100W`), `%FTP` / `% of 1mi pace`
/// targets, cadence (`@ 85rpm`), effort words (`free ride`, `walk`, `jog`,
/// `cooldown`). Targets stay notes in Phase 3 (zone/%FTP resolution needs
/// athlete thresholds — deferred like `run_base`/`bike_ftp`).
pub fn zwift() -> FormatSchema {
    let base = swimdojo();
    FormatSchema {
        name: "zwift".to_owned(),
        freestyle_words: vec![
            "free".to_owned(),
            "ride".to_owned(),
            "walk".to_owned(),
            "jog".to_owned(),
            "cooldown".to_owned(),
        ],
        drill_words: vec![],
        recovery_word: "easy".to_owned(),
        ..base
    }
}
///
/// Divergences from swimdojo, evidenced by 10 `Workout of the Week`
/// articles (see `scraper/tests/data/msp-*.html` + `documents/myswimpro.md`):
/// section labels (`Warmup`, `Pre-Set`, `Main Set`, `Cool Down`, plus
/// drill/set sub-sections like `Kick & Drill`, `IM Set`, `Scull Set`);
/// stroke-adjacent words (`kick`, `pull`, `drill` — kept as notes, since
/// the IDL has no equipment word); extra drill nouns (`scull`, `drills`);
/// a `×` (U+00D7) count separator alongside ` x `; rest spelled `rest`
/// with companions the parser keeps as notes.
pub fn myswimpro() -> FormatSchema {
    let base = swimdojo();
    FormatSchema {
        name: "myswimpro".to_owned(),
        section_labels: vec![
            ("warmup".to_owned(), SectionLabel::WarmUp),
            ("warm up".to_owned(), SectionLabel::WarmUp),
            ("pre-set".to_owned(), SectionLabel::Main),
            ("pre set".to_owned(), SectionLabel::Main),
            ("main set".to_owned(), SectionLabel::Main),
            ("main".to_owned(), SectionLabel::Main),
            ("cool down".to_owned(), SectionLabel::CoolDown),
            ("cooldown".to_owned(), SectionLabel::CoolDown),
            ("post-main".to_owned(), SectionLabel::Main),
        ],
        drill_words: vec![
            "bob".to_owned(),
            "bobs".to_owned(),
            "scull".to_owned(),
            "sculls".to_owned(),
            "drill".to_owned(),
            "drills".to_owned(),
        ],
        count_separator: "×".to_owned(),
        ..base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_json() {
        let schema = swimdojo();
        let json = serde_json::to_string_pretty(&schema).unwrap();
        let back: FormatSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(back, schema);
    }

    #[test]
    fn schema_drives_parser() {
        let schema = swimdojo();
        let pool = fit_core::Pool::yards25();
        let w = super::super::swimdojo::parse_with_schema(
            "Warm Up\n100 back\n4 x 50 @ 1:00\n",
            pool,
            None,
            &schema,
        )
        .unwrap();
        assert_eq!(w.sections.len(), 1);
        match &w.sections[0].steps[0] {
            fit_core::Step::Distance(d) => {
                assert_eq!(d.stroke, Some(Stroke::Back));
            }
            other => panic!("got {other:?}"),
        }
        match &w.sections[0].steps[1] {
            fit_core::Step::Repeat(r) => assert_eq!(r.count, 4),
            other => panic!("got {other:?}"),
        }
    }

    const MSP_1800: &str = include_str!("../../../documents/myswimpro-fixtures/1800-variety.txt");
    const MSP_COMEBACK: &str =
        include_str!("../../../documents/myswimpro-fixtures/comeback-1200.txt");
    const MSP_USRPT: &str = include_str!("../../../documents/myswimpro-fixtures/usrpt-1500.txt");
    const MSP_SCULL: &str = include_str!("../../../documents/myswimpro-fixtures/scull-1250.txt");

    #[test]
    fn myswimpro_schema_parses_site_fixtures() {
        let schema = myswimpro();
        let pool = fit_core::Pool::meters25();
        for text in [MSP_1800, MSP_COMEBACK, MSP_USRPT, MSP_SCULL] {
            let w = super::super::swimdojo::parse_with_schema(text, pool, None, &schema)
                .expect("myswimpro fixture parses");
            assert!(!w.flat_steps().is_empty());
        }
        // `×` separator, `s`-suffixed distances, drill nouns.
        let w = super::super::swimdojo::parse_with_schema(
            "Warmup\n4×50 Kick @ 1:10\n4 x 25s Freestyle @ :30\n",
            pool,
            None,
            &schema,
        )
        .unwrap();
        assert_eq!(w.flat_steps().len(), 8);
    }

    const RUN_W1: &str =
        include_str!("../../../documents/run-fixtures/higdon-5k-intermediate-w1.txt");
    const RUN_W2: &str =
        include_str!("../../../documents/run-fixtures/higdon-5k-intermediate-w2.txt");
    const RUN_10K_W2: &str =
        include_str!("../../../documents/run-fixtures/higdon-10k-intermediate-w2.txt");
    const BIKE_RAMP: &str =
        include_str!("../../../documents/bike-fixtures/zwift-ramp-test-excerpt.txt");
    const RUN_BEST_MILE: &str =
        include_str!("../../../documents/run-fixtures/zwift-101-w1-best-mile.txt");

    #[test]
    fn higdon_run_schema_parses_plan_fixtures() {
        let schema = higdon_run();
        let pool = fit_core::Pool::yards25();
        for text in [RUN_W1, RUN_W2, RUN_10K_W2] {
            let w = super::super::swimdojo::parse_with_schema(text, pool, None, &schema)
                .expect("higdon fixture parses");
            assert!(!w.flat_steps().is_empty());
        }
        // `Rest` day markers stay notes (no timed/distance step invented);
        // `3 mi run` keeps its explicit unit under the run schema too.
        let w = super::super::swimdojo::parse_with_schema(
            "Warm Up\n3 mi run\nRest\n",
            pool,
            None,
            &schema,
        )
        .unwrap();
        assert_eq!(w.flat_steps().len(), 1);
        assert_eq!(
            w.flat_steps()[0].distance,
            Some(fit_core::Distance::miles(3))
        );
    }

    #[test]
    fn zwift_schema_parses_interval_fixtures() {
        let schema = zwift();
        let pool = fit_core::Pool::yards25();
        for text in [BIKE_RAMP, RUN_BEST_MILE] {
            let w = super::super::swimdojo::parse_with_schema(text, pool, None, &schema)
                .expect("zwift fixture parses");
            assert!(!w.flat_steps().is_empty());
        }
        // `%FTP` / cadence targets stay notes in Phase 3 (no Target yet).
        let w = super::super::swimdojo::parse_with_schema(
            "Warm Up\n1min @ 85rpm, 100W\n",
            pool,
            None,
            &schema,
        )
        .unwrap();
        let flat = w.flat_steps();
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].time, Some(fit_core::Seconds::minutes(1)));
        let notes = flat[0].notes.as_deref().unwrap_or("");
        assert!(notes.contains("85rpm"), "{notes:?}");
        assert!(notes.contains("100W"), "{notes:?}");
    }
}
