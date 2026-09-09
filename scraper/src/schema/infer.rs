//! Infers a site's workout-format schema from sample notation texts.
//!
//! `infer` observes structural signals the parser itself keys on — section
//! labels, `TOTAL:` prefixes, repeat words, annotation markers, stroke and
//! drill words, interval/base/clock markers, rest and recovery words — and
//! emits the [`FormatSchema`] a generator consumes. It starts from the
//! [`swimdojo`](super::swimdojo) schema's structural defaults (markers,
//! rest/recovery words) and replaces the vocabulary tables with what the
//! samples evidence. Unknown tables stay at their defaults and are
//! reported in [`Inference.gaps`] so callers know what was guessed.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use fit_core::{FormatSchema, SectionLabel, Stroke};

use crate::site::ScrapedWorkout;

/// A workout-format schema inferred from samples, with provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inference {
    /// The inferred schema, ready to serialize or parse with.
    pub schema: FormatSchema,
    /// Tables with no sample evidence, kept at structural defaults.
    pub gaps: Vec<String>,
    /// `sample index -> lines the inference could not classify`.
    pub unclassified: Vec<(usize, Vec<String>)>,
}

impl fmt::Display for Inference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "schema '{}':", self.schema.name)?;
        if self.gaps.is_empty() {
            writeln!(f, "  evidence: all tables observed")?;
        } else {
            writeln!(f, "  gaps (defaults kept): {}", self.gaps.join(", "))?;
        }
        for (i, lines) in &self.unclassified {
            writeln!(f, "  sample {i} unclassified: {}", lines.join(" | "))?;
        }
        Ok(())
    }
}

/// Infer a [`FormatSchema`] named `name` from sample notation texts.
///
/// Each sample is one workout body (the `body` of a [`ScrapedWorkout`] or
/// a `documents/swimdojo-fixtures/*.txt` text). Every line the parser
/// would treat structurally (labels, totals, repeats, annotations, steps)
/// votes for its table; prose lines are ignored like the parser ignores
/// them. Tables with no votes keep swimdojo defaults and land in `gaps`.
pub fn infer(name: &str, samples: &[ScrapedWorkout]) -> Inference {
    infer_texts(
        name,
        &samples.iter().map(|w| w.body.as_str()).collect::<Vec<_>>(),
    )
}

/// [`infer`] over raw body texts (convenience for tests and the CLI).
pub fn infer_texts(name: &str, samples: &[&str]) -> Inference {
    let base = super::swimdojo_schema();
    let mut labels: BTreeMap<String, SectionLabel> = BTreeMap::new();
    let mut total_prefixes: BTreeMap<String, usize> = BTreeMap::new();
    let mut repeat_words: BTreeMap<String, usize> = BTreeMap::new();
    let mut markers: BTreeSet<String> = BTreeSet::new();
    let mut strokes: BTreeMap<String, Stroke> = BTreeMap::new();
    let mut freestyle: BTreeSet<String> = BTreeSet::new();
    let mut drills: BTreeSet<String> = BTreeSet::new();
    let mut clocks: BTreeSet<String> = BTreeSet::new();
    let mut bases: BTreeSet<String> = BTreeSet::new();
    let mut rests: BTreeSet<String> = BTreeSet::new();
    let mut recoveries: BTreeMap<String, usize> = BTreeMap::new();
    let mut unclassified: Vec<(usize, Vec<String>)> = Vec::new();

    for (idx, sample) in samples.iter().enumerate() {
        let mut stray = Vec::new();
        for raw in sample.lines() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            let low = line.to_ascii_lowercase();
            if let Some(label) = classify_label(&low) {
                labels.entry(label.0).or_insert(label.1);
            } else if low.starts_with("total") {
                total_prefixes
                    .entry(word_before_colon(&low).unwrap_or_else(|| "total".to_owned()))
                    .and_modify(|c| *c += 1)
                    .or_insert(1);
            } else if let Some(word) = repeat_word_of(&low) {
                repeat_words
                    .entry(word)
                    .and_modify(|c| *c += 1)
                    .or_insert(1);
            } else if let Some(marker) = annotation_marker_of(line) {
                markers.insert(marker);
                observe_interval_markers(line, &mut clocks, &mut bases);
                // Annotations quote step vocabulary (`800 IM (200 of each
                // stroke)`, `easy back`, `swim back`): observe their words
                // so stroke/drill tables get votes. Quoted counts
                // (`2 x 400`) are bare words here, never repeat blocks.
                observe_annotation_words(
                    line,
                    &mut strokes,
                    &mut freestyle,
                    &mut drills,
                    &mut rests,
                );
            } else if line.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                observe_interval_markers(line, &mut clocks, &mut bases);
                observe_step(
                    line,
                    &mut strokes,
                    &mut freestyle,
                    &mut drills,
                    &mut rests,
                    &mut recoveries,
                );
            } else {
                stray.push(line.to_owned());
            }
        }
        if !stray.is_empty() {
            unclassified.push((idx, stray));
        }
    }

    let mut gaps = Vec::new();
    let pick = |votes: BTreeMap<String, usize>, dflt: &str, gap: &str, gaps: &mut Vec<String>| {
        if votes.is_empty() {
            gaps.push(gap.to_owned());
            dflt.to_owned()
        } else {
            top(votes)
        }
    };
    let mut schema = base.clone();
    schema.name = name.to_owned();
    if labels.is_empty() {
        gaps.push("section_labels".to_owned());
    } else {
        schema.section_labels = labels.into_iter().collect();
    }
    schema.total_prefix = pick(
        total_prefixes,
        &base.total_prefix,
        "total_prefix",
        &mut gaps,
    );
    schema.repeat_word = pick(repeat_words, &base.repeat_word, "repeat_word", &mut gaps);
    if markers.is_empty() {
        gaps.push("annotation_markers".to_owned());
    } else {
        schema.annotation_markers = markers.into_iter().collect();
    }
    if strokes.is_empty() {
        gaps.push("strokes".to_owned());
    } else {
        schema.strokes = strokes.into_iter().collect();
    }
    if freestyle.is_empty() {
        gaps.push("freestyle_words".to_owned());
    } else {
        schema.freestyle_words = freestyle.into_iter().collect();
    }
    if drills.is_empty() {
        gaps.push("drill_words".to_owned());
    } else {
        schema.drill_words = drills.into_iter().collect();
    }
    if clocks.is_empty() {
        gaps.push("clock_prefix".to_owned());
    }
    if rests.is_empty() {
        gaps.push("rest_words".to_owned());
    } else {
        schema.rest_words = rests.into_iter().collect();
    }
    if recoveries.is_empty() {
        gaps.push("recovery_word".to_owned());
    } else {
        schema.recovery_word = top(recoveries);
    }
    if bases.is_empty() {
        gaps.push("base_marker".to_owned());
    }
    Inference {
        schema,
        gaps,
        unclassified,
    }
}

fn top(votes: BTreeMap<String, usize>) -> String {
    votes
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(w, _)| w)
        .expect("non-empty votes")
}

/// `Warm Up`, `Main Set`, `Set N`, `Warm Down` / `Cool Down` (and their
/// `:`-suffixed forms) — plus the myswimpro variants proven by the second
/// site (`Warmup`, `Pre-Set`, `Post-Main`, drill/set sub-sections, repeat
/// suffixes like `Main Set (3x)` / `IM Set (x2)`). Unknown `X Set` / drill
/// headings map to `Main` (a totaled group, like swimdojo's `Set N`).
fn classify_label(low: &str) -> Option<(String, SectionLabel)> {
    // Strip a repeat suffix first: `main set (3x)` → `main set`.
    let bare = low
        .split('(')
        .next()
        .unwrap_or(low)
        .trim_end_matches([':', ' '])
        .to_owned();
    let label = match bare.as_str() {
        "warm up" | "warmup" => SectionLabel::WarmUp,
        "warm down" | "cool down" | "cooldown" => SectionLabel::CoolDown,
        "main set" | "main" | "pre-set" | "pre set" | "post-main" => SectionLabel::Main,
        _ => {
            if let Some(n) = bare
                .strip_prefix("set ")
                .and_then(|rest| rest.trim().parse::<u32>().ok())
            {
                SectionLabel::Set(n)
            } else if !bare.chars().any(|c| c.is_ascii_digit())
                && ["set", "drill"].iter().any(|w| bare.ends_with(w))
            {
                SectionLabel::Main
            } else {
                return None;
            }
        }
    };
    Some((bare, label))
}

fn word_before_colon(low: &str) -> Option<String> {
    low.split(':')
        .next()
        .map(|w| w.trim().to_owned())
        .filter(|w| !w.is_empty() && w.chars().all(|c| c.is_ascii_alphabetic()))
}

/// `2x through:` / `2 x through:` — the trailing word is the repeat word.
/// The `Nx` count may be spaceless (`3x`), so fall back to splitting off
/// a leading-digit run when there is no whitespace. Counted steps
/// (`10 x 200`) and bare swims (`500 swim`) never match: a repeat block
/// opens with a count *and nothing else* besides the word.
fn repeat_word_of(low: &str) -> Option<String> {
    let bare = low.trim_end_matches([':', ' ']);
    if let Some((head, word)) = bare.rsplit_once(char::is_whitespace) {
        if !word.chars().all(|c| c.is_ascii_alphabetic()) || head.trim_end().is_empty() {
            return None;
        }
        // `N x WORD` with nothing else: the head must itself be just the
        // count (`2 x` / `2x`), otherwise this is a step (`500 swim` has
        // a bare-number head; `10 x 200` has a numeric tail already
        // excluded above).
        let head = head.trim_end();
        let count = head
            .strip_suffix(['x', '×'])
            .map(str::trim_end)
            .unwrap_or(head);
        if count.chars().all(|c| c.is_ascii_digit()) && !count.is_empty() && head != count {
            return Some(word.to_owned());
        }
        // Spaceless `Nx WORD` is a step (`3x10` never occurs); only the
        // no-whitespace form below can be a repeat word.
        return None;
    }
    let digits = bare
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>();
    let word = bare[digits.len()..]
        .strip_prefix(['x', '×'])
        .unwrap_or(&bare[digits.len()..]);
    if !digits.is_empty() && !word.is_empty() && word.chars().all(|c| c.is_ascii_alphabetic()) {
        Some(word.to_owned())
    } else {
        None
    }
}

/// Leading arrow runs (`—>`, `-->`, bare `—`) — longest match first.
fn annotation_marker_of(line: &str) -> Option<String> {
    for marker in ["—>", "–>", "-->", "--", "—", "–"] {
        if line.starts_with(marker) {
            return Some(marker.to_owned());
        }
    }
    None
}

/// First `@`-separated word of `line` into the clock/base sets
/// (`@ kb` ⇒ clock, `@ b…` ⇒ base) — for annotation lines, which carry
/// intervals but no step words.
fn observe_interval_markers(
    line: &str,
    clocks: &mut BTreeSet<String>,
    bases: &mut BTreeSet<String>,
) {
    if let Some((_, iv)) = line.split_once('@') {
        let low = iv.to_ascii_lowercase();
        if let Some(word) = low
            .split(|c: char| !c.is_ascii_alphabetic())
            .find(|w| !w.is_empty())
        {
            if word == "kb" {
                clocks.insert(word.to_owned());
            } else if word == "b" {
                bases.insert(word.to_owned());
            }
        }
    }
}

/// Observe a digit-led step line: stroke/drill words, interval markers,
/// rest and recovery words — mirroring the parser's own classifiers.
///
/// Annotation lines (`—>…`, e.g. Box Crab's `—>#1-3 kick @ kb`) carry
/// intervals too, so the `@` half is always scanned even when the left
/// half has no digit-led step.
fn observe_step(
    line: &str,
    strokes: &mut BTreeMap<String, Stroke>,
    freestyle: &mut BTreeSet<String>,
    drills: &mut BTreeSet<String>,
    rests: &mut BTreeSet<String>,
    recoveries: &mut BTreeMap<String, usize>,
) {
    let (left, _) = match line.split_once('@') {
        Some((l, r)) => (l, Some(r.trim())),
        None => (line, None),
    };
    let tokens: Vec<String> = left
        .split_whitespace()
        .flat_map(|t| {
            // `4×50` / `5x60m`: split spaceless counts into count +
            // distance tokens so `×`-separated sites vote correctly.
            let t = t.trim_matches(|c: char| !c.is_alphanumeric());
            match t.find(['×', 'x']) {
                Some(i)
                    if t[..i].chars().all(|c| c.is_ascii_digit())
                        && t[i + 1..].starts_with(|c: char| c.is_ascii_digit()) =>
                {
                    vec![t[..i].to_owned(), t[i + 1..].to_owned()]
                }
                _ => vec![t.to_ascii_lowercase()],
            }
        })
        .map(|t| {
            t.trim_matches(|c: char| !c.is_alphanumeric())
                .to_ascii_lowercase()
        })
        .filter(|t| !t.is_empty())
        .collect();
    if tokens.is_empty() {
        return;
    }
    let words = if tokens[0].chars().all(|c| c.is_ascii_digit()) {
        &tokens[1..]
    } else if tokens.len() > 2 && tokens[1] == "x" && tokens[0].chars().all(|c| c.is_ascii_digit())
    {
        &tokens[2..]
    } else if annotation_marker_of(line.trim()).is_some() {
        // Annotation line: no step words (intervals scanned by the caller).
        return;
    } else {
        return;
    };
    // Rest words live on either side of `@` (`30 seconds rest` on the
    // left; `9 x 50 @ :20 rest:` on the right), so check the whole line.
    // A drill line with an interval (`3 x 10 bobs … @ :30 rest …`) is
    // rest-bearing *and* drill-bearing: record both, then continue to
    // word observation instead of returning.
    let whole: Vec<String> = line
        .split(|c: char| !c.is_ascii_alphanumeric())
        .map(|t| t.to_ascii_lowercase())
        .filter(|t| !t.is_empty() && !t.chars().all(|c| c.is_ascii_digit()) && t != "x")
        .collect();
    if whole.iter().any(|w| w == "rest") {
        // `30 seconds rest`: every non-number token is a rest word. The
        // Sea Otter summary `9 x 50 @ :20 rest:` carries its companions
        // after the `@`, scanned in `whole` above. Drop tokens with
        // digits (`5x60m`, ranges like `15-30` split into `15`/`30` —
        // already filtered — but joined forms survive) and link boiler.
        rests.extend(
            whole
                .iter()
                .filter(|w| !w.chars().any(|c| c.is_ascii_digit()) && *w != "link" && *w != "here")
                .cloned(),
        );
        rests.insert("rest".to_owned());
    }
    if let Some(pos) = words.iter().position(|w| w == "easy") {
        recoveries
            .entry("easy".to_owned())
            .and_modify(|c| *c += 1)
            .or_insert(1);
        observe_words(&words[..pos], strokes, freestyle, drills);
        return;
    }
    observe_words(words, strokes, freestyle, drills);
}

/// Words inside an annotation line (`—>…`) that match the known step
/// vocabulary — annotations quote it (`800 IM`, `easy back`, `swim back`,
/// `scull down`). Same word lists as [`observe_words`]; `rest` companions
/// join the rest table (prose noise like `odds`/`evens` is left out by the
/// known-word filter).
fn observe_annotation_words(
    line: &str,
    strokes: &mut BTreeMap<String, Stroke>,
    freestyle: &mut BTreeSet<String>,
    drills: &mut BTreeSet<String>,
    rests: &mut BTreeSet<String>,
) {
    let words: Vec<String> = line
        .split(|c: char| !c.is_ascii_alphanumeric())
        .map(|t| t.to_ascii_lowercase())
        .filter(|t| !t.is_empty())
        .collect();
    let before = strokes.len();
    observe_words(&words, strokes, freestyle, drills);
    if words.iter().any(|w| w == "rest") {
        rests.insert("rest".to_owned());
    }
    let _ = before;
}

/// Sort step words into strokes, freestyle defaults, and drill nouns using
/// the same word lists the parser matches (the site under inference is a
/// swim site until proven otherwise; unseen words stay unclassified).
fn observe_words(
    words: &[String],
    strokes: &mut BTreeMap<String, Stroke>,
    freestyle: &mut BTreeSet<String>,
    drills: &mut BTreeSet<String>,
) {
    const KNOWN_STROKES: [(&str, Stroke); 9] = [
        ("back", Stroke::Back),
        ("backstroke", Stroke::Back),
        ("breast", Stroke::Breast),
        ("breaststroke", Stroke::Breast),
        ("fly", Stroke::Fly),
        ("butterfly", Stroke::Fly),
        ("im", Stroke::IM),
        ("imedley", Stroke::IM),
        ("stroke", Stroke::Any),
    ];
    const KNOWN_FREESTYLE: [&str; 3] = ["free", "freestyle", "swim"];
    const KNOWN_DRILLS: [&str; 6] = ["bob", "bobs", "scull", "sculls", "drill", "drills"];
    for w in words {
        if let Some((_, s)) = KNOWN_STROKES.iter().find(|(k, _)| k == w) {
            strokes.entry(w.clone()).or_insert(*s);
        } else if KNOWN_FREESTYLE.contains(&w.as_str()) {
            freestyle.insert(w.clone());
        } else if KNOWN_DRILLS.contains(&w.as_str()) {
            drills.insert(w.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOBLIN: &str = include_str!("../../../documents/swimdojo-fixtures/goblin-shark.txt");
    const BOX_CRAB: &str = include_str!("../../../documents/swimdojo-fixtures/box-crab.txt");
    const SEA_OTTER: &str = include_str!("../../../documents/swimdojo-fixtures/sea-otter.txt");

    fn samples() -> Vec<ScrapedWorkout> {
        [GOBLIN, BOX_CRAB, SEA_OTTER]
            .into_iter()
            .enumerate()
            .map(|(i, body)| ScrapedWorkout {
                title: format!("sample {i}"),
                url: format!("https://example.com/{i}"),
                body: body.to_owned(),
            })
            .collect()
    }

    #[test]
    fn infers_swimdojo_vocabulary() {
        let inf = infer("swimdojo", &samples());
        let s = &inf.schema;
        assert!(inf.gaps.is_empty(), "gaps: {:?}", inf.gaps);
        for label in ["warm up", "main set", "warm down"] {
            assert!(
                s.section_labels.iter().any(|(l, _)| l == label),
                "{label} in {:?}",
                s.section_labels
            );
        }
        assert!(
            s.section_labels
                .iter()
                .any(|(_, l)| matches!(l, SectionLabel::Set(_)))
        );
        assert_eq!(s.total_prefix, "total");
        assert_eq!(s.repeat_word, "through");
        assert!(s.annotation_markers.contains(&"—>".to_owned()));
        assert_eq!(s.stroke_of("back"), Some(Stroke::Back));
        assert_eq!(s.stroke_of("im"), Some(Stroke::IM));
        assert_eq!(s.stroke_of("stroke"), Some(Stroke::Any));
        assert!(s.is_freestyle("swim"));
        assert!(s.is_drill("bobs"));
        assert_eq!(s.clock_prefix, "kb");
        assert_eq!(s.base_marker, "b");
        assert!(s.rest_words.contains(&"rest".to_owned()));
        assert_eq!(s.recovery_word, "easy");
    }

    #[test]
    fn inferred_schema_parses_fixtures() {
        let inf = infer("swimdojo", &samples());
        assert!(inf.gaps.is_empty());
        for text in [GOBLIN, BOX_CRAB, SEA_OTTER] {
            let a = fit_generator::parse_str(text, fit_core::Pool::yards25(), None).unwrap();
            let b = fit_generator::parser::swimdojo::parse_with_schema(
                text,
                fit_core::Pool::yards25(),
                None,
                &inf.schema,
            )
            .unwrap();
            assert_eq!(a, b);
        }
    }

    #[test]
    fn empty_samples_report_gaps() {
        let inf = infer_texts("empty", &["Warm Up\n"]);
        assert!(!inf.gaps.is_empty());
        assert!(inf.gaps.contains(&"strokes".to_owned()));
    }
}
