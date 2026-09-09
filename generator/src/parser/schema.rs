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
}
