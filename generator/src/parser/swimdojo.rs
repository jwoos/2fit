//! Parser for swimdojo.com workout notation (grammar in
//! `documents/swimdojo-grammar.txt`, real examples in
//! `documents/swimdojo-fixtures/`).
//!
//! The notation is distance-based (`100 swim`, `4 x 100 @ 2:00`) and
//! unit-agnostic: the [`Pool`] passed to [`parse`] supplies the unit.
//!
//! Line kinds, in recognition order:
//!
//! - `TOTAL: N` — the stated workout total; ignored (sections keep their
//!   own stated subtotals).
//! - Section labels — `Warm Up`, `Main` / `Main Set`, `Set N`,
//!   `Warm Down` / `Cool Down` (case-insensitive, trailing `:` allowed).
//! - Subtotals — a whole-line number: parenthesized `(1300)` or bare. A
//!   bare number is a subtotal only when the current section already has
//!   steps; otherwise it is a single freestyle swim of that distance.
//!   (In swimdojo HTML subtotals are `<em>` tags; once tags are stripped
//!   the after-steps position is the only reliable signal.)
//! - `N x through:` — opens a repeat block; following lines are its inner
//!   steps until the next label, subtotal, or end. A block that collected
//!   no steps (Sea Otter's `3x through:` followed only by `—>` notes) is
//!   dropped: the set's summary line (`9 x 50 @ :20 rest:`) already is
//!   the repeated step.
//! - `—>` / `–>` / `-->` annotations — attached as a note to the most
//!   recent step that has a notes slot (never parsed as steps).
//! - A line starting with a digit — a step.
//! - Anything else — prose, attached as a note to the previous step (or
//!   dropped when there is no previous step).

use std::fmt;

use fit_core::{
    DistanceStep, IntervalSpec, Pool, RecoveryStep, RepeatStep, Seconds, Section, SectionLabel,
    Step, Stroke, TechniqueStep, Workout,
};

/// A line of notation that could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// 1-based line number in the input text.
    pub line: usize,
    /// The offending line, trimmed.
    pub text: String,
    /// Why the line could not be parsed.
    pub reason: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: '{}': {}", self.line, self.text, self.reason)
    }
}

impl std::error::Error for Error {}

/// Parse swimdojo workout notation into a [`Workout`].
///
/// `pool` supplies the distance unit; `base100` is the swimmer's base per
/// 100 (required later to resolve `@ b` intervals, see
/// [`IntervalSpec::resolved`]). Line-level failures return [`Error`] with
/// the line number; other lines are never silently dropped except prose
/// with no step to attach to.
pub fn parse(text: &str, pool: Pool, base100: Option<Seconds>) -> Result<Workout, Error> {
    let mut w = Workout::new(pool);
    w.base100 = base100;
    let mut through: Option<RepeatStep> = None;

    for (idx, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let n = idx + 1;

        if is_total(line) {
            continue;
        }

        if let Some(label) = label_of(line) {
            close_through(&mut through, &mut w.sections);
            w.sections.push(Section {
                label,
                steps: Vec::new(),
                subtotal: None,
            });
            continue;
        }

        if let Some((value, parenthesized)) = bare_number(line) {
            let through_open = through.is_some();
            let has_steps = w.sections.last().is_some_and(|s| !s.steps.is_empty());
            if parenthesized || (has_steps && !through_open) {
                close_through(&mut through, &mut w.sections);
                let subtotal = w.dist(value);
                let section = w
                    .sections
                    .last_mut()
                    .ok_or_else(|| err(n, line, "subtotal before a section label"))?;
                section.subtotal = Some(subtotal);
            } else if through_open {
                return Err(err(n, line, "number inside a 'x through' block"));
            } else {
                let step = Step::Distance(DistanceStep {
                    distance: w.dist(value),
                    stroke: None,
                    interval: None,
                    intensity: Default::default(),
                    notes: None,
                });
                push(&mut through, &mut w.sections, step);
            }
            continue;
        }

        if let Some(count) = through_count(line) {
            close_through(&mut through, &mut w.sections);
            ensure_section(&mut w.sections);
            through = Some(RepeatStep {
                count,
                rest_between: None,
                inner: Vec::new(),
                notes: None,
            });
            continue;
        }

        if let Some(note) = annotation_text(line) {
            note_to(
                through
                    .as_mut()
                    .map(|t| t.inner.as_mut_slice())
                    .filter(|s| !s.is_empty()),
                &mut w.sections,
                note,
            );
            continue;
        }

        if line
            .chars()
            .next()
            .is_some_and(|c: char| c.is_ascii_digit())
        {
            let step = parse_step(&w, line, n)?;
            push(&mut through, &mut w.sections, step);
            continue;
        }

        note_to(
            through
                .as_mut()
                .map(|t| t.inner.as_mut_slice())
                .filter(|s| !s.is_empty()),
            &mut w.sections,
            line,
        );
    }

    close_through(&mut through, &mut w.sections);
    Ok(w)
}

fn ensure_section(sections: &mut Vec<Section>) {
    if sections.is_empty() {
        sections.push(Section {
            label: SectionLabel::None,
            steps: Vec::new(),
            subtotal: None,
        });
    }
}

fn push(through: &mut Option<RepeatStep>, sections: &mut Vec<Section>, step: Step) {
    if let Some(t) = through.as_mut() {
        t.inner.push(step);
    } else {
        ensure_section(sections);
        sections.last_mut().unwrap().steps.push(step);
    }
}

/// Close an open `x through` block, pushing it as a step if it collected
/// any (an annotations-only block is dropped — see module docs).
fn close_through(through: &mut Option<RepeatStep>, sections: &mut Vec<Section>) {
    if let Some(t) = through.take()
        && !t.inner.is_empty()
    {
        ensure_section(sections);
        sections.last_mut().unwrap().steps.push(Step::Repeat(t));
    }
}

/// Attach a note to the inner steps of an open repeat block, or — when the
/// block has no steps yet — to the current section's steps (Sea Otter's
/// `3x through:` case). Dropped when nothing can take it.
fn note_to(inner: Option<&mut [Step]>, sections: &mut [Section], note: &str) {
    let Some(steps) = inner.or_else(|| sections.last_mut().map(|s| s.steps.as_mut_slice())) else {
        return;
    };
    attach(steps, note);
}

fn attach(steps: &mut [Step], note: &str) {
    for step in steps.iter_mut().rev() {
        let target = match step {
            Step::Distance(d) => &mut d.notes,
            Step::Breakdown(b) => &mut b.notes,
            Step::Recovery(r) => &mut r.notes,
            Step::Repeat(r) => &mut r.notes,
            Step::Technique(t) => &mut t.notes,
            Step::Rest { .. } => continue,
            _ => continue,
        };
        *target = Some(match target {
            Some(prev) => format!("{prev} {note}"),
            None => note.to_string(),
        });
        return;
    }
}

/// `TOTAL: 6,000` / `Total: 6,000` — the stated workout total.
fn is_total(line: &str) -> bool {
    line.to_ascii_lowercase().starts_with("total")
}

fn label_of(line: &str) -> Option<SectionLabel> {
    let lower = line.to_ascii_lowercase();
    let low = lower.trim_end_matches([':', ' ']);
    if low == "warm up" || low == "warmup" {
        Some(SectionLabel::WarmUp)
    } else if low == "warm down" || low == "cool down" || low == "cooldown" {
        Some(SectionLabel::CoolDown)
    } else if low == "main set" || low == "main" {
        Some(SectionLabel::Main)
    } else {
        low.strip_prefix("set ")
            .and_then(|rest| rest.trim().parse::<u32>().ok())
            .map(SectionLabel::Set)
    }
}

/// Whole-line number, optionally parenthesized and comma-grouped.
fn bare_number(line: &str) -> Option<(u32, bool)> {
    let inner = if let Some(r) = line.strip_prefix('(') {
        r.strip_suffix(')')?
    } else {
        line
    };
    let parenthesized = line.starts_with('(');
    let digits = inner.replace(',', "");
    if digits.is_empty() || !digits.chars().all(|c: char| c.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok().map(|v| (v, parenthesized))
}

/// `2x through` / `2 x through` (case-insensitive, trailing `:` allowed).
fn through_count(line: &str) -> Option<u32> {
    let lower = line.to_ascii_lowercase();
    let low = lower.trim().trim_end_matches(':');
    let head = low.strip_suffix("through")?;
    let head = head.trim_end().strip_suffix("x")?;
    let n = head.trim_end();
    if n.is_empty() || !n.chars().all(|c| c.is_ascii_digit() || c.is_whitespace()) {
        return None;
    }
    n.replace(' ', "").parse().ok()
}

/// Annotation (`—>...`): return the text after the arrow.
fn annotation_text(line: &str) -> Option<&str> {
    line.strip_prefix("—>")
        .or_else(|| line.strip_prefix("–>"))
        .or_else(|| line.strip_prefix("-->"))
        .or_else(|| line.strip_prefix("--"))
        .or_else(|| line.strip_prefix("—"))
        .or_else(|| line.strip_prefix("–"))
        .map(str::trim)
}

/// One step line: optional `N x` count, a leading number, optional stroke
/// and drill words, optional `@ interval`, optional `—>` tail annotation.
fn parse_step(w: &Workout, line: &str, n: usize) -> Result<Step, Error> {
    let mut tail_annotation: Option<String> = None;
    let body = split_arrow(line, &mut tail_annotation);

    let (count, body) = split_count(body);
    let (left, interval_text) = match body.split_once('@') {
        Some((l, r)) => (l, Some(r.trim())),
        None => (body, None),
    };
    let left = left.trim().trim_end_matches([':', ',', '.']);

    let mut toks = left.split_whitespace();
    let first = toks
        .next()
        .ok_or_else(|| err(n, line, "expected a distance"))?;
    let value = numeric(first)
        .ok_or_else(|| err(n, line, format!("expected a distance, got '{first}'")))?;
    let rest: Vec<&str> = toks.collect();

    let (interval, interval_note) = parse_interval(interval_text, n, line)?;
    let lower: Vec<String> = rest.iter().map(|t| t.to_ascii_lowercase()).collect();

    // Rep-counted drills: `3 x 10 bobs`, `10 sculls` — the leading number
    // is reps, not distance (see `TechniqueStep` docs).
    const DRILLS: [&str; 4] = ["bob", "bobs", "scull", "sculls"];
    if let Some(pos) = lower.iter().position(|t| DRILLS.contains(&t.as_str())) {
        let drill = rest[pos];
        let mut words = Vec::new();
        words.extend_from_slice(&rest[..pos]);
        words.extend_from_slice(&rest[pos + 1..]);
        let notes = join_notes(words, interval_note, tail_annotation);
        return Ok(Step::Technique(TechniqueStep {
            drill: drill.to_string(),
            reps_per_set: Some(value),
            sets: count,
            interval,
            notes,
        }));
    }

    let inner: Step = if lower.iter().any(|t| t == "rest")
        // `30 seconds rest` — a rest step, not a swim.
        && lower
            .iter()
            .all(|t| matches!(t.as_str(), "rest" | "second" | "seconds"))
    {
        Step::Rest {
            secs: Seconds::secs(value),
        }
    } else if lower.iter().any(|t| t == "easy") {
        // `50 easy` — active recovery.
        let (stroke, words) = take_stroke(&lower, &rest);
        let mut notes = join_notes(words, interval_note, tail_annotation);
        // `easy` steps have no interval field; keep the stated one in notes.
        if let Some(iv) = interval {
            notes = Some(match notes {
                Some(existing) => format!("{existing} {iv}"),
                None => iv.to_string(),
            });
        }
        Step::Recovery(RecoveryStep {
            distance: w.dist(value),
            stroke,
            notes,
        })
    } else {
        let (stroke, words) = take_stroke(&lower, &rest);
        Step::Distance(DistanceStep {
            distance: w.dist(value),
            stroke,
            interval,
            intensity: Default::default(),
            notes: join_notes(words, interval_note, tail_annotation),
        })
    };

    // A leading `N x` makes the step a repeated one.
    match count {
        None => Ok(inner),
        Some(c) => Ok(Step::Repeat(RepeatStep {
            count: c,
            rest_between: None,
            inner: vec![inner],
            notes: None,
        })),
    }
}

/// Split a step line at the first `—>`-style arrow in its interior:
/// `100 swim strong, breathing every three—> see what your time is`.
fn split_arrow<'a>(line: &'a str, annotation: &mut Option<String>) -> &'a str {
    for arrow in ["—>", "–>", "-->"] {
        if let Some(pos) = line.find(arrow) {
            let (before, after) = line.split_at(pos);
            let note = after[arrow.len()..].trim();
            if !note.is_empty() {
                *annotation = Some(note.to_string());
            }
            return before.trim_end();
        }
    }
    line
}

/// Peel an `N x ` / `Nx` count prefix, e.g. `4 x 100` → `(4, "100")`.
fn split_count(body: &str) -> (Option<u32>, &str) {
    if let Some((n, rest)) = body.split_once(" x ")
        && let Some(count) = numeric(n)
    {
        return (Some(count), rest.trim_start());
    }
    if let Some(pos) = body.find(['x', '×']) {
        let (head, tail) = body.split_at(pos);
        if head.chars().all(|c: char| c.is_ascii_digit())
            && !head.is_empty()
            && tail.starts_with(char::is_whitespace)
            && let Some(count) = numeric(head)
        {
            return (Some(count), body[pos + 1..].trim_start());
        }
    }
    (None, body)
}

fn numeric(tok: &str) -> Option<u32> {
    let d = tok.trim().trim_end_matches([':', '.', ',']);
    d.replace(',', "").parse().ok().filter(|&v: &u32| v > 0)
}

/// `@ ...` text → `(interval, leftover-note)`.
///
/// - `kb` → not modeled in the IDL; kept as a note.
/// - `b`, `b±5`, `b +2:00`, `b +:+30` → [`IntervalSpec::Base`].
/// - `2:00`, `:30`, `45` → [`IntervalSpec::Fixed`].
fn parse_interval(
    text: Option<&str>,
    n: usize,
    line: &str,
) -> Result<(Option<IntervalSpec>, Option<String>), Error> {
    let Some(text) = text.filter(|t| !t.is_empty()) else {
        return Ok((None, None));
    };
    let low = text.to_ascii_lowercase();

    if low.starts_with("kb") {
        return Ok((None, Some(text.to_string())));
    }

    if let Some(off) = low.strip_prefix('b') {
        let off = off.trim();
        let (interval, tail) = if off.is_empty() {
            (IntervalSpec::base(), off)
        } else {
            let (positive, mag) = if let Some(m) = off.strip_prefix('+') {
                (true, m)
            } else if let Some(m) = off.strip_prefix('-') {
                (false, m)
            } else {
                (true, off)
            };
            let (secs, tail) = take_time_prefix(mag)
                .ok_or_else(|| err(n, line, format!("unparseable base offset in '@ {text}'")))?;
            let offset: i32 = secs.try_into().unwrap_or(i32::MAX);
            (
                IntervalSpec::base_offset(if positive { offset } else { -offset }),
                tail,
            )
        };
        let note = tail.trim().trim_start_matches([' ', ',', ':']);
        return Ok((
            Some(interval),
            if note.is_empty() {
                None
            } else {
                Some(note.to_string())
            },
        ));
    }

    let (secs, tail) = take_time_prefix(text)
        .ok_or_else(|| err(n, line, format!("unparseable interval '@ {text}'")))?;
    let note = tail.trim().trim_start_matches([' ', ',', ':']);
    Ok((
        Some(IntervalSpec::Fixed(Seconds::secs(secs))),
        if note.is_empty() {
            None
        } else {
            Some(note.to_string())
        },
    ))
}

/// Leading time (`:ss` / `m:ss` / bare seconds) of `s` plus the remainder.
fn take_time_prefix(s: &str) -> Option<(u32, &str)> {
    if let Some(rest) = s.strip_prefix(':') {
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        let secs = rest[..end].parse().ok()?;
        return Some((secs, &rest[end..]));
    }
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    if end == 0 {
        return None;
    }
    let minutes = s[..end].parse::<u32>().ok()?;
    let rest = &s[end..];
    if let Some(rest) = rest.strip_prefix(':') {
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        let secs = rest[..end].parse::<u32>().ok()?;
        return Some((minutes * 60 + secs, &rest[end..]));
    }
    Some((minutes, rest))
}

/// Stroke word(s) in the token list; everything else is note text.
/// `free`/`swim` are the freestyle default: no stroke, no note.
fn take_stroke<'a>(lower: &'a [String], rest: &'a [&'a str]) -> (Option<Stroke>, Vec<&'a str>) {
    let mut stroke = None;
    let mut words = Vec::new();
    for (i, t) in lower.iter().enumerate() {
        match t.as_str() {
            "free" | "swim" => {}
            "back" | "backstroke" => stroke = Some(Stroke::Back),
            "breast" | "breaststroke" => stroke = Some(Stroke::Breast),
            "fly" | "butterfly" => stroke = Some(Stroke::Fly),
            "im" | "imedley" => stroke = Some(Stroke::IM),
            "stroke" => stroke = Some(Stroke::Any),
            _ => words.push(rest[i]),
        }
    }
    (stroke, words)
}

fn join_notes(
    words: Vec<&str>,
    interval_note: Option<String>,
    tail_annotation: Option<String>,
) -> Option<String> {
    let mut parts: Vec<String> = words.iter().map(|w| w.to_string()).collect();
    if let Some(p) = interval_note {
        parts.push(p);
    }
    if let Some(p) = tail_annotation {
        parts.push(p);
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn err(n: usize, text: &str, reason: impl Into<String>) -> Error {
    Error {
        line: n,
        text: text.to_string(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fit_core::Distance;

    const GOBLIN: &str = include_str!("../../../documents/swimdojo-fixtures/goblin-shark.txt");
    const BOX_CRAB: &str = include_str!("../../../documents/swimdojo-fixtures/box-crab.txt");
    const SEA_OTTER: &str = include_str!("../../../documents/swimdojo-fixtures/sea-otter.txt");

    fn pool() -> Pool {
        Pool::yards25()
    }

    #[test]
    fn goblin_shark() {
        let w = parse(GOBLIN, pool(), None).expect("parses");
        assert_eq!(w.sections.len(), 3);

        let warm = &w.sections[0];
        assert_eq!(warm.label, SectionLabel::WarmUp);
        assert_eq!(warm.steps.len(), 4);
        assert_eq!(warm.subtotal, Some(Distance::yards(1300)));
        assert_eq!(warm.total_distance(), 1300);

        let main = &w.sections[1];
        assert_eq!(main.label, SectionLabel::Main);
        assert_eq!(main.steps.len(), 8);
        // Stated subtotal (4200) intentionally differs from the step sum
        // (4000); it is stored as stated, never recomputed.
        assert_eq!(main.subtotal, Some(Distance::yards(4200)));
        assert_eq!(main.total_distance(), 4000);

        // `800 IM @ b +2:00` → IM on base + 120 s.
        match &main.steps[1] {
            Step::Distance(d) => {
                assert_eq!(d.distance, Distance::yards(800));
                assert_eq!(d.stroke, Some(Stroke::IM));
                assert_eq!(d.interval, Some(IntervalSpec::base_offset(120)));
            }
            other => panic!("expected distance step, got {other:?}"),
        }
        // `200 IM @ b +:+30` → base + 30 s; the trailing prose note kept.
        match &main.steps[7] {
            Step::Distance(d) => {
                assert_eq!(d.interval, Some(IntervalSpec::base_offset(30)));
                assert_eq!(
                    d.notes.as_deref(),
                    Some(
                        "Adjust your IM bases as needed. Make the IMs the focus of the set and push the base if you'd like, use the free as recovery."
                    )
                );
            }
            other => panic!("expected distance step, got {other:?}"),
        }

        assert_eq!(w.sections[2].label, SectionLabel::Set(2));
        assert_eq!(w.sections[2].steps.len(), 2);
        assert_eq!(w.total_distance(), Distance::yards(6000));
    }

    #[test]
    fn goblin_shark_interval_trailing_note() {
        let w = parse(
            "100 IM @ b +1:30 (options for how to swim same as above)\n",
            pool(),
            None,
        )
        .expect("parses");
        match &w.sections[0].steps[0] {
            Step::Distance(d) => {
                assert_eq!(d.interval, Some(IntervalSpec::base_offset(90)));
                assert_eq!(
                    d.notes.as_deref(),
                    Some("(options for how to swim same as above)")
                );
            }
            other => panic!("got {other:?}"),
        }
    }

    #[test]
    fn box_crab() {
        let w = parse(BOX_CRAB, pool(), None).expect("parses");
        assert_eq!(w.sections.len(), 4);
        assert_eq!(w.sections[1].label, SectionLabel::Set(1));
        assert_eq!(w.sections[2].label, SectionLabel::Main);
        assert_eq!(w.sections[3].label, SectionLabel::Set(3));

        // `10 x 200` with per-rep-range split notes.
        match &w.sections[1].steps[0] {
            Step::Repeat(r) => {
                assert_eq!(r.count, 10);
                let notes = r.notes.as_deref().expect("split notes");
                assert!(notes.contains("#1-3 kick @ kb"), "{notes}");
                assert!(notes.contains("#4-6 pull @ b"), "{notes}");
                assert!(notes.contains("#7-10 swim @ b +10"), "{notes}");
                assert!(notes.contains("100 IM/100 free"), "{notes}");
            }
            other => panic!("expected repeat step, got {other:?}"),
        }
        assert_eq!(w.sections[1].subtotal, Some(Distance::yards(2000)));

        // `30 x 100` with pace notes.
        match &w.sections[2].steps[0] {
            Step::Repeat(r) => {
                assert_eq!(r.count, 30);
                let notes = r.notes.as_deref().expect("pace notes");
                assert!(notes.contains("First 10 @ b+40"), "{notes}");
                assert!(notes.contains("Last 20 @ b+20"), "{notes}");
                assert!(notes.contains("WITHOUT FALLING OFF"), "{notes}");
            }
            other => panic!("expected repeat step, got {other:?}"),
        }
        assert_eq!(w.sections[2].subtotal, Some(Distance::yards(3000)));

        // `10 x 50 @ b +20`.
        match &w.sections[3].steps[0] {
            Step::Repeat(r) => {
                assert_eq!(r.count, 10);
                match &r.inner[0] {
                    Step::Distance(d) => {
                        assert_eq!(d.distance, Distance::yards(50));
                        assert_eq!(d.interval, Some(IntervalSpec::base_offset(20)));
                    }
                    other => panic!("got {other:?}"),
                }
            }
            other => panic!("expected repeat step, got {other:?}"),
        }

        assert_eq!(w.total_distance(), Distance::yards(6000));
    }

    #[test]
    fn sea_otter() {
        let w = parse(SEA_OTTER, pool(), None).expect("parses");
        assert_eq!(w.sections.len(), 4);
        assert_eq!(w.sections[3].label, SectionLabel::CoolDown);

        let warm = &w.sections[0];
        assert_eq!(warm.steps.len(), 3);
        // `3 x 10 bobs (link here) @ :30 rest in between each set` —
        // rep-counted drill: 0 swim distance.
        match &warm.steps[2] {
            Step::Technique(t) => {
                assert_eq!(t.drill, "bobs");
                assert_eq!(t.reps_per_set, Some(10));
                assert_eq!(t.sets, Some(3));
                assert_eq!(t.interval, Some(IntervalSpec::Fixed(Seconds::secs(30))));
                let notes = t.notes.as_deref().expect("drill notes");
                assert!(notes.contains("link here"), "{notes}");
                assert!(notes.contains("rest in between each set"), "{notes}");
            }
            other => panic!("expected technique step, got {other:?}"),
        }

        // Set 1: `9 x 50 @ :20 rest:` is the repeated step; the
        // annotations-only `3x through:` block is dropped and its notes
        // attach to the 9x50.
        assert_eq!(w.sections[1].steps.len(), 1);
        match &w.sections[1].steps[0] {
            Step::Repeat(r) => {
                assert_eq!(r.count, 9);
                match &r.inner[0] {
                    Step::Distance(d) => {
                        assert_eq!(d.interval, Some(IntervalSpec::Fixed(Seconds::secs(20))));
                    }
                    other => panic!("got {other:?}"),
                }
                let notes = r.notes.as_deref().expect("breathing notes");
                assert!(
                    notes.contains("breathing every 4 to your comfortable side"),
                    "{notes}"
                );
                assert!(
                    notes.contains("breathing every 3, alternating sides"),
                    "{notes}"
                );
                assert!(
                    notes.contains("focus on your breathing patterns"),
                    "{notes}"
                );
            }
            other => panic!("expected repeat step, got {other:?}"),
        }

        // `100 swim strong, breathing every three—> see what your time is`
        // splits at the embedded arrow.
        match &w.sections[2].steps[0] {
            Step::Distance(d) => {
                let notes = d.notes.as_deref().expect("step notes");
                assert!(notes.contains("strong"), "{notes}");
                assert!(notes.contains("see what your time is"), "{notes}");
            }
            other => panic!("got {other:?}"),
        }

        // 100 + 100 + 0 (bobs) + 450 + 100 + 50 = the stated TOTAL: 800.
        assert_eq!(w.total_distance(), Distance::yards(800));
    }

    #[test]
    fn grammar_warmup_subtotal() {
        let text = "Warm Up\n500 swim\n200 kick\n300 pull breathing 3/5/7 by 100\n1000\n";
        let w = parse(text, pool(), None).expect("parses");
        assert_eq!(w.sections.len(), 1);
        assert_eq!(w.sections[0].subtotal, Some(Distance::yards(1000)));
        assert_eq!(w.sections[0].total_distance(), 1000);
        // `300 pull breathing 3/5/7 by 100` — pull is a note, not a stroke.
        match &w.sections[0].steps[2] {
            Step::Distance(d) => {
                assert_eq!(d.stroke, None);
                assert_eq!(d.notes.as_deref(), Some("pull breathing 3/5/7 by 100"));
            }
            other => panic!("got {other:?}"),
        }
    }

    #[test]
    fn grammar_set_with_stroke() {
        let text = "Set 1\n8 x 50 @ b\n4 x 25 stroke @ :30\n500\n";
        let w = parse(text, pool(), Some(Seconds::secs(120))).expect("parses");
        assert_eq!(w.sections[0].subtotal, Some(Distance::yards(500)));
        assert_eq!(w.sections[0].total_distance(), 500);
        match &w.sections[0].steps[0] {
            Step::Repeat(r) => {
                assert_eq!(r.count, 8);
                match &r.inner[0] {
                    Step::Distance(d) => {
                        assert_eq!(d.interval, Some(IntervalSpec::base()));
                    }
                    other => panic!("got {other:?}"),
                }
            }
            other => panic!("got {other:?}"),
        }
        match &w.sections[0].steps[1] {
            Step::Repeat(r) => match &r.inner[0] {
                Step::Distance(d) => {
                    // "stroke" = anything but freestyle.
                    assert_eq!(d.stroke, Some(Stroke::Any));
                    assert_eq!(d.interval, Some(IntervalSpec::Fixed(Seconds::secs(30))));
                }
                other => panic!("got {other:?}"),
            },
            other => panic!("got {other:?}"),
        }
    }

    #[test]
    fn grammar_repeats() {
        let text = "2x through:\n4 x 100 @ 2:00\n6 x 50 @ 1:00\n30 seconds rest\n";
        let w = parse(text, pool(), None).expect("parses");
        assert_eq!(w.sections.len(), 1);
        assert_eq!(w.sections[0].steps.len(), 1);
        assert_eq!(w.sections[0].total_distance(), 1400); // 2 × (400 + 300)
        match &w.sections[0].steps[0] {
            Step::Repeat(r) => {
                assert_eq!(r.count, 2);
                assert_eq!(r.inner.len(), 3);
                assert!(matches!(
                    r.inner[2],
                    Step::Rest {
                        secs
                    } if secs == Seconds::secs(30)
                ));
            }
            other => panic!("got {other:?}"),
        }
    }

    #[test]
    fn grammar_easy_swim() {
        let text = "2x through:\n4 x 100 @ b +5\n6 x 50 @ b\n50 easy\n";
        let w = parse(text, pool(), Some(Seconds::secs(120))).expect("parses");
        let flat = w.flat_steps();
        // 2 × (4 + 6 + 1) = 22 swum steps (the 50 easy repeats too).
        assert_eq!(flat.len(), 22);
        match &w.sections[0].steps[0] {
            Step::Repeat(r) => {
                assert_eq!(r.count, 2);
                match &r.inner[0] {
                    Step::Repeat(inner) => match &inner.inner[0] {
                        Step::Distance(d) => {
                            assert_eq!(d.interval, Some(IntervalSpec::base_offset(5)));
                        }
                        other => panic!("got {other:?}"),
                    },
                    other => panic!("got {other:?}"),
                }
                match r.inner.last().expect("recovery last") {
                    Step::Recovery(recovery) => {
                        assert_eq!(recovery.distance, Distance::yards(50));
                    }
                    other => panic!("got {other:?}"),
                }
            }
            other => panic!("got {other:?}"),
        }
        // Base math with 2:00 per 100: 200 @ b-5 = 3:55.
        let w = parse("200 @ b-5\n", pool(), Some(Seconds::secs(120))).expect("parses");
        match &w.sections[0].steps[0] {
            Step::Distance(d) => {
                let secs = d
                    .interval
                    .as_ref()
                    .unwrap()
                    .resolved(d.distance, Some(Seconds::secs(120)))
                    .unwrap();
                assert_eq!(secs, Seconds::secs(235));
            }
            other => panic!("got {other:?}"),
        }
    }

    #[test]
    fn bare_number_rules() {
        // A bare number with no previous step is a swim, not a subtotal.
        let w = parse("100\n", pool(), None).expect("parses");
        assert_eq!(w.sections[0].steps.len(), 1);
        assert_eq!(w.sections[0].subtotal, None);
        // A bare number after steps in a section is the subtotal.
        let w = parse("100\n200\n500\n", pool(), None).expect("parses");
        assert_eq!(w.sections[0].steps.len(), 1);
        assert_eq!(w.sections[0].subtotal, Some(Distance::yards(500)));
        // Parenthesized is always a subtotal.
        let w = parse("(500)\n", pool(), None).is_err();
        assert!(w, "subtotal before any section errors");
    }

    #[test]
    fn errors() {
        let e = parse("100 @ qqq\n", pool(), None).unwrap_err();
        assert_eq!(e.line, 1);
        assert!(e.reason.contains("interval"), "{}", e.reason);
        assert!(e.to_string().contains("line 1"));

        let e = parse("(500)\n100\n", pool(), None).unwrap_err();
        assert_eq!(e.line, 1);
        assert!(e.reason.contains("subtotal"), "{}", e.reason);
    }

    #[test]
    fn leading_prose_with_no_step_is_dropped() {
        let w = parse("today: legs\n100\n", pool(), None).expect("parses");
        assert_eq!(w.sections.len(), 1);
        assert_eq!(w.sections[0].steps.len(), 1);
    }

    #[test]
    fn prose_attaches_to_previous_step() {
        let w = parse("100\nswim it well\n", pool(), None).expect("parses");
        match &w.sections[0].steps[0] {
            Step::Distance(d) => {
                assert_eq!(d.notes.as_deref(), Some("swim it well"));
            }
            other => panic!("got {other:?}"),
        }
    }
}
