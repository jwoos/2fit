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
    Distance, DistanceStep, FormatSchema, IntervalSpec, Pace, Pool, RecoveryStep, RepeatStep,
    Seconds, Section, SectionLabel, Step, Stroke, Target, TechniqueStep, TimedStep, Workout,
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

/// Athlete thresholds for target resolution (mirrors [`Workout`]'s
/// `run_base`/`bike_ftp`; passed separately so `parse` signatures stay
/// swim-shaped — the seed `Workout` carries swim's `base100` only).
#[derive(Debug, Clone, Copy, Default)]
pub struct Thresholds {
    /// Run threshold pace (resolves `% of pace` → [`Target::Pace`]).
    pub run_base: Option<Pace>,
    /// Bike FTP watts (resolves `%FTP` → [`Target::Power`]).
    pub bike_ftp: Option<u32>,
}

/// Parse swimdojo workout notation into a [`Workout`].
///
/// `pool` supplies the distance unit; `base100` is the swimmer's base per
/// 100 (required later to resolve `@ b` intervals, see
/// [`IntervalSpec::resolved`]). Line-level failures return [`Error`] with
/// the line number; other lines are never silently dropped except prose
/// with no step to attach to.
pub fn parse(text: &str, pool: Pool, base100: Option<Seconds>) -> Result<Workout, Error> {
    parse_with_schema(text, pool, base100, &super::schema::swimdojo())
}

/// Parse notation driven by a [`FormatSchema`]'s vocabulary tables.
///
/// [`parse`] is this with the swimdojo schema; a future site passes its
/// own schema value. Structural rules (subtotal positions, repeat blocks,
/// note attachment) are format-shared; only the words come from the schema.
pub fn parse_with_schema(
    text: &str,
    pool: Pool,
    base100: Option<Seconds>,
    schema: &FormatSchema,
) -> Result<Workout, Error> {
    parse_with_thresholds(text, pool, base100, Thresholds::default(), schema)
}

/// Parse notation with athlete thresholds for `%`-target resolution.
///
/// [`parse_with_schema`] is this with no thresholds (all `%`-targets stay
/// notes); pass `--run-base`/`--ftp` values here to resolve them.
pub fn parse_with_thresholds(
    text: &str,
    pool: Pool,
    base100: Option<Seconds>,
    thresholds: Thresholds,
    schema: &FormatSchema,
) -> Result<Workout, Error> {
    let mut w = Workout::new(pool);
    w.base100 = base100;
    w.run_base = thresholds.run_base;
    w.bike_ftp = thresholds.bike_ftp;
    let mut through: Option<RepeatStep> = None;

    for (idx, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let n = idx + 1;

        if is_total(line, schema) {
            continue;
        }

        if let Some(label) = label_of(line, schema) {
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
                    target: None,
                    intensity: Default::default(),
                    notes: None,
                });
                push(&mut through, &mut w.sections, step);
            }
            continue;
        }

        if let Some(count) = through_count(line, schema) {
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

        if let Some(note) = annotation_text(line, schema) {
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
            let step = parse_step(&w, line, n, schema)?;
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
fn is_total(line: &str, schema: &FormatSchema) -> bool {
    line.to_ascii_lowercase().starts_with(&schema.total_prefix)
}

fn label_of(line: &str, schema: &FormatSchema) -> Option<SectionLabel> {
    let lower = line.to_ascii_lowercase();
    if let Some(label) = schema.section_of(&lower) {
        return Some(label);
    }
    lower
        .strip_prefix("set ")
        .and_then(|rest| rest.trim().parse::<u32>().ok())
        .map(SectionLabel::Set)
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

/// `2x through:` / `2 x through:` (case-insensitive, trailing `:` allowed).
fn through_count(line: &str, schema: &FormatSchema) -> Option<u32> {
    let lower = line.to_ascii_lowercase();
    let low = lower.trim().trim_end_matches(':');
    let head = low.strip_suffix(schema.repeat_word.as_str())?;
    let head = head.trim_end().strip_suffix("x")?;
    let n = head.trim_end();
    if n.is_empty() || !n.chars().all(|c| c.is_ascii_digit() || c.is_whitespace()) {
        return None;
    }
    n.replace(' ', "").parse().ok()
}

/// Annotation (`—>...`): return the text after the arrow.
fn annotation_text<'a>(line: &'a str, schema: &FormatSchema) -> Option<&'a str> {
    schema
        .annotation_markers
        .iter()
        .find_map(|m| line.strip_prefix(m.as_str()))
        .map(str::trim)
}

/// One step line: optional `N x` count, a leading number, optional stroke
/// and drill words, optional `@ interval`, optional `—>` tail annotation.
fn parse_step(w: &Workout, line: &str, n: usize, schema: &FormatSchema) -> Result<Step, Error> {
    let mut tail_annotation: Option<String> = None;
    let body = split_arrow(line, &mut tail_annotation, schema);

    let (count, body) = split_count(body, schema);
    let (left, interval_text) = match body.split_once(schema.interval_marker.as_str()) {
        Some((l, r)) => (l, Some(r.trim())),
        None => (body, None),
    };
    let left = left.trim().trim_end_matches([':', ',', '.']);

    let mut toks = left.split_whitespace();
    let first = toks
        .next()
        .ok_or_else(|| err(n, line, "expected a distance"))?;
    // Leading durations (`35 min tempo run`, `60 min cross`): run/bike work
    // prescribed by time. A bare `m:ss`/`h:mm:ss` first token is a duration
    // too (`30:00 easy`). Both become `Step::Timed` with no target — the
    // trailing words (`tempo`, `fast`, `cross`) stay notes, matching the
    // Higdon evidence (effort lives in prose, not the cell).
    if let Some(duration) = leading_duration(first, &mut toks) {
        let rest: Vec<&str> = toks.collect();
        // `@` after a *duration* is a target (`1min @ 85rpm, 100W`,
        // `20min @ 110% FTP`), never a swim interval — parse it with the
        // workout's thresholds (`bike_ftp`/`run_base`) instead of
        // `parse_interval` (which would misread `85rpm` as a `b`-style base:
        // `take_time_prefix("85rpm")` yields 85 + tail `rpm`).
        if interval_text.is_some_and(|t| !t.is_empty()) {
            let t = interval_text.unwrap_or("").trim();
            let (target, leftover) = parse_target_text(t, w);
            let mut words: Vec<String> = rest
                .iter()
                .filter(|t| !is_run_filler(&t.to_ascii_lowercase()))
                .map(|t| t.to_string())
                .collect();
            if !leftover.is_empty() {
                words.push(format!("@ {leftover}"));
            }
            let notes = join_notes(words, None, tail_annotation);
            let inner = Step::Timed(TimedStep {
                duration,
                target,
                intensity: Default::default(),
                notes,
            });
            return match count {
                None => Ok(inner),
                Some(c) => Ok(Step::Repeat(RepeatStep {
                    count: c,
                    rest_between: None,
                    inner: vec![inner],
                    notes: None,
                })),
            };
        }
        let (interval, interval_note) = parse_interval(interval_text, n, line, schema)?;
        let mut notes = join_notes(
            rest.iter()
                .filter(|t| !is_run_filler(&t.to_ascii_lowercase()))
                .map(|t| t.to_string())
                .collect(),
            interval_note,
            tail_annotation,
        );
        if let Some(iv) = interval {
            notes = Some(match notes {
                Some(existing) => format!("{existing} {iv}"),
                None => iv.to_string(),
            });
        }
        let inner = Step::Timed(TimedStep {
            duration,
            target: None,
            intensity: Default::default(),
            notes,
        });
        return match count {
            None => Ok(inner),
            Some(c) => Ok(Step::Repeat(RepeatStep {
                count: c,
                rest_between: None,
                inner: vec![inner],
                notes: None,
            })),
        };
    }
    // Explicit run/bike units (`3 mi run`, `4.8 km run`, `8 x 400 5K pace`):
    // the unit beats the pool. Decimal km (`4.8`) parse to whole meters;
    // `400` with a pace word stays meters (track reps).
    if let Some(distance) = suffixed_distance(first, &mut toks) {
        let rest: Vec<&str> = toks.collect();
        return finish_run_distance(
            w,
            line,
            n,
            schema,
            distance,
            rest,
            interval_text,
            tail_annotation,
            count,
        );
    }
    // Unit-suffixed distances (`60m`, `50s`): strip one trailing length
    // letter; the pool supplies the unit, `s` is drill shorthand.
    let first = first.strip_suffix(['m', 'M', 's']).unwrap_or(first);
    let value = numeric(first)
        .ok_or_else(|| err(n, line, format!("expected a distance, got '{first}'")))?;
    let rest: Vec<&str> = toks.collect();

    let (interval, interval_note) = parse_interval(interval_text, n, line, schema)?;
    let lower: Vec<String> = rest.iter().map(|t| t.to_ascii_lowercase()).collect();

    // Rep-counted drills: `3 x 10 bobs`, `10 sculls` — the leading number
    // is reps, not distance (see `TechniqueStep` docs).
    if let Some(pos) = lower.iter().position(|t| schema.is_drill(t)) {
        let drill = rest[pos];
        let words: Vec<String> = rest[..pos]
            .iter()
            .chain(&rest[pos + 1..])
            .map(|s| s.to_string())
            .collect();
        let notes = join_notes(words, interval_note, tail_annotation);
        return Ok(Step::Technique(TechniqueStep {
            drill: drill.to_string(),
            reps_per_set: Some(value),
            sets: count,
            interval,
            notes,
        }));
    }

    // Rest detection: the step must *be* a rest prescription (`30 seconds
    // rest`), not merely mention rest words. `lower` holds only the
    // post-distance tokens, so require every one to be rest vocabulary
    // *and* the literal `rest` token to be present: `300 Freestyle ...`
    // has `lower` = [`freestyle`] — no `rest` token, so it stays a swim
    // even when an inferred schema lists `freestyle` as a rest companion
    // (from `5x60m Freestyle, 15-30 seconds rest` evidence). The marker is
    // the literal word `rest`, not `rest_words[0]` (unordered after
    // inference — BTreeSet order put `freestyle` first here).
    let inner: Step = if lower.iter().any(|t| t == "rest")
        && lower
            .iter()
            .all(|t| schema.rest_words.iter().any(|w| w == t))
    {
        Step::Rest {
            secs: Seconds::secs(value),
        }
    } else if lower.contains(&schema.recovery_word) {
        // `50 easy` — active recovery.
        let (stroke, words) = take_stroke(&lower, &rest, schema);
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
        let (stroke, words) = take_stroke(&lower, &rest, schema);
        Step::Distance(DistanceStep {
            distance: w.dist(value),
            stroke,
            interval,
            target: None,
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
fn split_arrow<'a>(
    line: &'a str,
    annotation: &mut Option<String>,
    schema: &FormatSchema,
) -> &'a str {
    for arrow in schema
        .annotation_markers
        .iter()
        .filter(|m| m.ends_with('>'))
    {
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

/// Peel an `N x ` / `Nx` / `N×M` count prefix, e.g. `4 x 100` → `(4, "100")`.
/// The schema's separator is tried first (` x ` for swimdojo, `×` for
/// myswimpro); spaceless `Nx` and `N×M` forms always work.
fn split_count<'a>(body: &'a str, schema: &FormatSchema) -> (Option<u32>, &'a str) {
    if let Some((n, rest)) = body.split_once(schema.count_separator.as_str())
        && let Some(count) = numeric(n)
    {
        return (Some(count), rest.trim_start());
    }
    for sep in [" x ", "×", "x"] {
        if sep == schema.count_separator {
            continue;
        }
        if let Some((n, rest)) = body.split_once(sep)
            && let Some(count) = numeric(n)
        {
            // ` x `/`×` need nothing more; bare `x` must not split words
            // (`6×50` is fine, `max effort` is not a count).
            if sep != "x" || rest.starts_with(char::is_whitespace) || n.contains('×') {
                return (Some(count), rest.trim_start());
            }
        }
    }
    if let Some(pos) = body.find(['x', '×']) {
        let (head, tail) = body.split_at(pos);
        // Spaceless `NxM` (`5x60m`): the distance may carry a unit suffix.
        let head_digits = head.chars().all(|c: char| c.is_ascii_digit()) && !head.is_empty();
        if head_digits && let Some(count) = numeric(head) {
            let after = tail[1..].trim_start();
            // `5x60m …` → distance token `60m` (suffix stripped below);
            // `3x through:` has no digit after `x` → not a count.
            if after.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                return (Some(count), after);
            }
            if tail.starts_with(char::is_whitespace) {
                return (Some(count), body[pos + 1..].trim_start());
            }
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
    schema: &FormatSchema,
) -> Result<(Option<IntervalSpec>, Option<String>), Error> {
    let Some(text) = text.filter(|t| !t.is_empty()) else {
        return Ok((None, None));
    };
    let low = text.to_ascii_lowercase();

    if low.starts_with(schema.clock_prefix.as_str()) {
        return Ok((None, Some(text.to_string())));
    }

    if let Some(off) = low.strip_prefix(schema.base_marker.as_str()) {
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
/// Schema freestyle words are the silent default: no stroke, no note.
fn take_stroke(
    lower: &[String],
    rest: &[&str],
    schema: &FormatSchema,
) -> (Option<Stroke>, Vec<String>) {
    let mut stroke = None;
    let mut words = Vec::new();
    for (i, t) in lower.iter().enumerate() {
        if schema.is_freestyle(t) {
            continue;
        }
        if let Some(s) = schema.stroke_of(t) {
            stroke = Some(s);
        } else {
            words.push(rest[i].to_owned());
        }
    }
    (stroke, words)
}

fn join_notes(
    words: Vec<String>,
    interval_note: Option<String>,
    tail_annotation: Option<String>,
) -> Option<String> {
    let mut parts: Vec<String> = words;
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

/// Words that fill run cells but carry no IDL meaning: the sport noun
/// itself (`run`), filler (`cross` in `60 min cross` — the day marker, kept
/// as a note only when other words justify it; see below), and pace
/// qualifiers (`5K pace` is a named effort, not yet a `Target` — Phase 3
/// keeps it as a note; `run_base` resolution is deferred).
fn is_run_filler(word: &str) -> bool {
    matches!(word, "run" | "runs" | "cross" | "x-train" | "xtrain")
}

/// A leading time token: `<N> min|mins|minute|minutes` or spaceless `<N>min`
/// (consumes the unit word from `toks` when separate) or bare `m:ss` /
/// `h:mm:ss`. Returns the duration.
/// Evidence: Higdon 10K Intermediate cells (`35 min tempo run`,
/// `60 min cross`, `8 x 400 5K pace` is the distance sibling); Zwift
/// bike cells (`5min free ride`, `1min @ 85rpm, 100W` — spaceless).
fn leading_duration<'a>(
    first: &str,
    toks: &mut (impl Iterator<Item = &'a str> + Clone),
) -> Option<Seconds> {
    // Spaceless `<N>min...` (`5min`, `1min` — Zwift): the unit is attached.
    let low_first = first.to_ascii_lowercase();
    for suffix in ["minutes", "minute", "mins", "min"] {
        if let Some(num) = low_first.strip_suffix(suffix)
            && !num.is_empty()
            && let Some(mins) = numeric(num)
        {
            return Some(Seconds::minutes(mins));
        }
    }
    if let Some(mins) = numeric(first) {
        // Peek the unit without consuming on mismatch: collect is avoided
        // by cloning the iterator (slice-backed, cheap).
        let mut peek = toks.clone();
        if let Some(unit) = peek.next()
            && matches!(
                unit.to_ascii_lowercase().as_str(),
                "min" | "mins" | "minute" | "minutes"
            )
        {
            let _ = toks.next();
            return Some(Seconds::minutes(mins));
        }
        return None;
    }
    if first.contains(':') && first.chars().all(|c| c.is_ascii_digit() || c == ':') {
        return time_token(first);
    }
    None
}

/// `m:ss` / `h:mm:ss` → seconds. `None` on bad shape (`sec >= 60`,
/// empty parts, overflow).
fn time_token(tok: &str) -> Option<Seconds> {
    let parts: Vec<&str> = tok.split(':').collect();
    let (h, m, s) = match parts.as_slice() {
        [m, s] => (0u32, m.parse::<u32>().ok()?, s.parse::<u32>().ok()?),
        [h, m, s] => (
            h.parse::<u32>().ok()?,
            m.parse::<u32>().ok()?,
            s.parse::<u32>().ok()?,
        ),
        _ => return None,
    };
    if m >= 60 || s >= 60 {
        return None;
    }
    h.checked_mul(3600)?
        .checked_add(m.checked_mul(60)?)?
        .checked_add(s)
        .map(Seconds)
}

/// An explicit run/bike distance starting at `first`, consuming a following
/// unit word when present. Returns the distance in its own unit (pool is
/// ignored — the unit beats the pool):
///
/// - `3 mi [run]` / `3.5 mi [run]` → miles (decimal miles → whole meters
///   via 1 mi = 1609.344 m, integer round — Higdon's `3.5 mi` Tuesday runs);
///   `4.8 km [run]` → decimal km → whole meters;
///   `400 [5K pace]` / `5 x 400` → bare meters (track reps);
/// - `N` + `m|meter|meters` → meters; `N` + `km|kilometer(s)` → km→m;
///   `N` + `mi|mile(s)` → miles.
/// - `None` when `first` is not a run distance (non-numeric, or a bare
///   integer with no unit/pace context — those stay pool-unit swims).
fn suffixed_distance<'a>(
    first: &str,
    toks: &mut (impl Iterator<Item = &'a str> + Clone),
) -> Option<Distance> {
    // Attached suffixes: `400m`, `5km`, `3mi`, `5k` (= km, Higdon `5K pace`
    // shorthand appears bare too — handled below via lookahead).
    let low = first.to_ascii_lowercase();
    for suffix in [
        "kilometers",
        "kilometer",
        "meters",
        "meter",
        "miles",
        "mile",
        "km",
        "mi",
        "m",
    ] {
        if let Some(num) = low.strip_suffix(suffix)
            && !num.is_empty()
        {
            if suffix == "km" || suffix.starts_with("kilo") {
                if num.contains('.') {
                    return Distance::from_km_decimal(num);
                }
                let km: u32 = num.replace(',', "").parse().ok()?;
                if km == 0 {
                    return None;
                }
                return Some(Distance::meters(km.checked_mul(1000)?));
            }
            let unit = if suffix.starts_with("mi") {
                fit_core::Unit::Miles
            } else {
                fit_core::Unit::Meters
            };
            let value: u32 = num.replace(',', "").parse().ok()?;
            if value == 0 {
                return None;
            }
            return Some(Distance { value, unit });
        }
    }
    // `5k` = 5 km (Higdon shorthand, no `m`).
    if let Some(num) = low.strip_suffix('k')
        && !num.is_empty()
        && num
            .bytes()
            .all(|b| b.is_ascii_digit() || b == b',' || b == b'.')
    {
        if num.contains('.') {
            return Distance::from_km_decimal(num);
        }
        let km: u32 = num.replace(',', "").parse().ok()?;
        if km == 0 {
            return None;
        }
        return Some(Distance::meters(km.checked_mul(1000)?));
    }
    // Separate unit word: `3 mi [run]`, `4.8 km [run]`, `400 m`.
    let mut peek = toks.clone();
    let unit_word = peek.next()?.to_ascii_lowercase();
    match unit_word.as_str() {
        "mi" | "mile" | "miles" => {
            let _ = toks.next();
            Distance::from_miles_decimal(first)
        }
        "km" | "kilometer" | "kilometers" | "k" => {
            let _ = toks.next();
            if first.contains('.') {
                Distance::from_km_decimal(first)
            } else {
                let km = numeric(first)?;
                Some(Distance::meters(km.checked_mul(1000)?))
            }
        }
        "m" | "meter" | "meters" => {
            let value = numeric(first)?;
            let _ = toks.next();
            Some(Distance::meters(value))
        }
        // `8 x 400 5K pace`: bare meters + pace qualifier. The `400` arrives
        // here after `split_count` peeled `8 x`; `5k`/`5k pace` follows.
        // (Single `"k"` is covered by the km arm above.)
        "pace" => {
            let value = numeric(first)?;
            Some(Distance::meters(value))
        }
        _ => {
            // Look one further: `400 5K pace` — unit word is `5k`, not `pace`.
            if unit_word.len() >= 2 {
                let uw = unit_word.trim_end_matches(['.', ',', ':']);
                if uw.ends_with('k') && uw[..uw.len() - 1].bytes().all(|b| b.is_ascii_digit()) {
                    let value = numeric(first)?;
                    return Some(Distance::meters(value));
                }
            }
            None
        }
    }
}

/// Finish a run/bike explicit-unit distance step: `@` text is a target
/// (parsed with the workout's thresholds), not a swim clock; leftover
/// target words and pace qualifiers (`5K pace`) stay notes; filler drops.
#[allow(clippy::too_many_arguments)]
fn finish_run_distance(
    w: &Workout,
    _line: &str,
    _n: usize,
    schema: &FormatSchema,
    distance: Distance,
    rest: Vec<&str>,
    interval_text: Option<&str>,
    tail_annotation: Option<String>,
    count: Option<u32>,
) -> Result<Step, Error> {
    let _ = schema;
    let mut words: Vec<String> = rest
        .iter()
        .filter(|t| !is_run_filler(&t.to_ascii_lowercase()))
        .map(|t| t.to_string())
        .collect();
    // Trailing pace qualifiers resolve against `run_base` (`400 5K pace`
    // needs a race-pace map — still notes; `@ 85% of 1mi pace` resolves).
    let mut target: Option<Target> = None;
    words.retain(|word| {
        if target.is_none()
            && let Some(t) = trailing_pace_target(word, distance, w.run_base)
        {
            target = Some(t);
            return false;
        }
        true
    });
    if let Some(t) = interval_text
        && !t.is_empty()
    {
        let (parsed, leftover) = parse_target_text(t, w);
        if target.is_none() {
            target = parsed;
        } else if let Some(p) = parsed {
            words.push(format!("@ {p}"));
        }
        if !leftover.is_empty() {
            words.push(format!("@ {leftover}"));
        }
    }
    let notes = join_notes(words, None, tail_annotation);
    let inner = Step::Distance(DistanceStep {
        distance,
        stroke: None,
        interval: None,
        target,
        intensity: Default::default(),
        notes,
    });
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

/// Parse `@`-target text with the workout's thresholds. Returns
/// `(target, leftover)`: `target` is the resolved [`Target`] (or `None`
/// when nothing resolved), `leftover` the unresolved words (kept as notes).
///
/// Forms (evidence: Zwift excerpts in `run-bike-notation.md` §2):
/// `250W` / `100W` → watts; `@ 85rpm, 100W` → last-wins (power beats
/// cadence when both present — the device's primary target); `Z4`/`z2` →
/// HR zone (direct, needs no threshold); `95rpm` → cadence; `110% FTP`
/// → `bike_ftp × pct` (needs `--ftp`); `85% of 1mi pace` → `run_base`
/// scaled (needs `--run-base`); `5:00/km` or `5:00 /km` → pace.
/// `5K pace` (named race pace, no map) and ramps (`from 30 to 70% FTP`)
/// stay notes — `leftover`.
///
/// Percent semantics: `%FTP` scales watts up (`110%` of 250 W = 275 W);
/// `% of 1mi pace` scales pace *down* in speed terms — 85% effort ≈
/// `run_base / 0.85` secs/km (slower pace, fewer secs would be faster).
fn parse_target_text(t: &str, w: &Workout) -> (Option<Target>, String) {
    let mut target: Option<(Target, &str)> = None;
    let mut leftover: Vec<&str> = Vec::new();
    // Split `85rpm, 100W` / `110% of 1mi pace, 200 m @ 70%` on commas and
    // `@` (the `4x 200 m @ 110% …, 200 m @ 70% …` shape re-splits on `@`).
    // Last-wins: a later target supersedes; the beaten source word stays in
    // notes, never silently dropped. The winner's own word is not notes.
    let parts: Vec<&str> = t
        .split([',', '@'])
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    let mut i = 0;
    while i < parts.len() {
        let part = parts[i];
        // `from 30 to 70% FTP` ramps: keep whole (needs ≥2 parts).
        if part.to_ascii_lowercase().starts_with("from ") && parts[i..].join(" ").contains('%') {
            leftover.extend(parts[i..].iter().copied());
            break;
        }
        if let Some(resolved) = target_word(part, w) {
            // A later target supersedes: the beaten source word joins leftover.
            if let Some((_, prev_src)) = target.replace((resolved, part)) {
                leftover.push(prev_src);
            }
        } else {
            leftover.push(part);
        }
        i += 1;
    }
    let target = target.map(|(t, _)| t);
    (target, leftover.join(", "))
}

/// One `@`-target word → [`Target`]. `None` = not a target (stays notes).
fn target_word(part: &str, w: &Workout) -> Option<Target> {
    let low = part.to_ascii_lowercase();
    let compact: String = low.chars().filter(|c| !c.is_whitespace()).collect();
    // `Z4` → HR zone (direct; device resolves bounds from threshold).
    if compact.len() == 2
        && compact.starts_with('z')
        && let Some(z) = compact[1..].parse::<u8>().ok()
        && (1..=7).contains(&z)
    {
        return Some(Target::HrZone(z));
    }
    // `95rpm` → cadence.
    if let Some(num) = compact.strip_suffix("rpm")
        && let Ok(rpm) = num.parse::<u8>()
        && rpm > 0
    {
        return Some(Target::Cadence(rpm));
    }
    // `250W` → watts.
    if let Some(num) = compact.strip_suffix('w')
        && !num.is_empty()
        && num.bytes().all(|b| b.is_ascii_digit())
        && let Ok(watts) = num.parse::<u32>()
        && watts > 0
    {
        return Some(Target::Power(watts));
    }
    // `110% FTP` / `110%ftp` → bike_ftp × pct.
    if let Some((pct, _)) = compact.split_once('%')
        && let Ok(p) = pct.parse::<u32>()
        && compact.contains("ftp")
    {
        let ftp = w.bike_ftp?;
        return Some(Target::Power(ftp * p / 100));
    }
    // `85% of 1mi pace` / `85%of1mipace` → run_base / 0.85.
    if compact.contains('%') && compact.contains("pace") {
        let pct: u32 = compact.split('%').next()?.parse().ok()?;
        if pct == 0 {
            return None;
        }
        let base = w.run_base?.as_secs_per_km() as u64;
        return Some(Target::Pace(Pace::from_secs_per_km(
            (base * 100 / pct as u64) as u32,
        )));
    }
    // `5:00/km` → pace.
    if let Some((clock, _)) = compact.split_once('/')
        && let Some(secs) = clock_time(clock)
    {
        return Some(Target::Pace(Pace::from_secs_per_km(secs)));
    }
    None
}

/// Trailing pace qualifier on a distance line (`400 … 5K pace` is named —
/// still notes; `@ 85% of 1mi pace`-style trailing `% … pace` resolves via
/// `run_base`). `distance` is unused today (a future race-pace map keys
/// `5K`-style names off it); kept for the call shape.
fn trailing_pace_target(word: &str, _distance: Distance, run_base: Option<Pace>) -> Option<Target> {
    let low = word.to_ascii_lowercase();
    if !low.contains('%') || !low.contains("pace") {
        return None;
    }
    let pct: u32 = low.split('%').next()?.trim().parse().ok()?;
    if pct == 0 {
        return None;
    }
    let base = run_base?.as_secs_per_km() as u64;
    Some(Target::Pace(Pace::from_secs_per_km(
        (base * 100 / pct as u64) as u32,
    )))
}

/// `m:ss` clock → seconds (`5:00` → 300). `None` on bad shape.
fn clock_time(tok: &str) -> Option<u32> {
    let (m, s) = tok.split_once(':')?;
    let m: u32 = m.parse().ok()?;
    let s: u32 = s.parse().ok()?;
    if s >= 60 {
        return None;
    }
    m.checked_mul(60)?.checked_add(s)
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
        assert_eq!(w.total_distance(), Some(Distance::yards(6000)));
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

        assert_eq!(w.total_distance(), Some(Distance::yards(6000)));
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
        assert_eq!(w.total_distance(), Some(Distance::yards(800)));
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

    #[test]
    fn threshold_targets_resolve() {
        use fit_core::Target;
        let thresholds = Thresholds {
            run_base: Some(Pace::from_secs_per_km(300)), // 5:00/km
            bike_ftp: Some(250),
        };
        let schema = crate::parser::schema::zwift();
        let w = parse_with_thresholds(
            "Warm Up\n20min @ 110% FTP\n5min @ 85% of 1mi pace\n1min @ 250W\n3min @ Z4\n2min @ 95rpm\n10min from 75 to 70W\n",
            pool(),
            None,
            thresholds,
            &schema,
        )
        .expect("parses");
        let flat = w.flat_steps();
        assert_eq!(flat.len(), 6);
        // 110% of 250 W = 275 W.
        assert_eq!(flat[0].target, Some(Target::Power(275)));
        // 85% effort on 5:00/km → 300/0.85 = 352 s/km.
        assert_eq!(
            flat[1].target,
            Some(Target::Pace(Pace::from_secs_per_km(352)))
        );
        assert_eq!(flat[2].target, Some(Target::Power(250)));
        assert_eq!(flat[3].target, Some(Target::HrZone(4)));
        assert_eq!(flat[4].target, Some(Target::Cadence(95)));
        // Ramps stay notes (no single target).
        assert_eq!(flat[5].target, None);
        assert!(
            flat[5].notes.as_deref().unwrap_or("").contains("75"),
            "{flat:?}"
        );
        // Distance lines carry targets too (`800 m @ 85% of 1mi pace`).
        let w = parse_with_thresholds(
            "Warm Up\n800 m @ 85% of 1mi pace\n",
            pool(),
            None,
            thresholds,
            &schema,
        )
        .unwrap();
        let flat = w.flat_steps();
        assert_eq!(
            flat[0].target,
            Some(Target::Pace(Pace::from_secs_per_km(352)))
        );
        assert_eq!(flat[0].distance, Some(Distance::meters(800)));
        // Without thresholds the same lines stay notes (back-compat).
        let w = parse_with_schema("Warm Up\n20min @ 110% FTP\n", pool(), None, &schema).unwrap();
        assert_eq!(w.flat_steps()[0].target, None);
    }

    #[test]
    fn run_cells_with_explicit_units() {
        // Higdon 10K Intermediate cells (metric + imperial views of one grid).
        let w = parse(
            "3 mi run\n4.8 km run\n8 x 400 5K pace\n35 min tempo run\n60 min cross\n30:00 easy\n",
            pool(),
            None,
        )
        .expect("parses");
        let flat = w.flat_steps();
        // 3mi + 4800m + 8×400m + 35:00 + 60:00 + 30:00 = 13 flat steps.
        assert_eq!(flat.len(), 13);
        assert_eq!(flat[0].distance, Some(Distance::miles(3)));
        assert_eq!(flat[1].distance, Some(Distance::meters(4800)));
        assert_eq!(flat[2].distance, Some(Distance::meters(400)));
        assert_eq!(flat[2].notes.as_deref(), Some("5K pace"));
        assert_eq!(flat[10].time, Some(Seconds::minutes(35)));
        assert_eq!(flat[10].notes.as_deref(), Some("tempo"));
        assert_eq!(flat[11].time, Some(Seconds::minutes(60)));
        assert_eq!(flat[12].time, Some(Seconds::minutes(30)));
        // Pool-unit swims still parse (unit path only triggers on units).
        let w = parse("100\n4 x 50 @ 1:00\n", pool(), None).expect("parses");
        assert_eq!(w.flat_steps().len(), 5);
    }

    #[test]
    fn run_unit_forms() {
        // Attached, separate, decimal, and `k` shorthand.
        for (line, want) in [
            ("5km\n", Distance::meters(5000)),
            ("5k\n", Distance::meters(5000)),
            ("400m\n", Distance::meters(400)),
            ("3 miles\n", Distance::miles(3)),
            ("3.5 mi run\n", Distance::meters(5633)),
            ("11.3 km run\n", Distance::meters(11_300)),
        ] {
            let w = parse(line, pool(), None).expect("parses");
            match &w.sections[0].steps[0] {
                Step::Distance(d) => assert_eq!(d.distance, want, "{line:?}"),
                other => panic!("got {other:?} for {line:?}"),
            }
        }
        // `@` after a run distance is a note, never a swim interval.
        let w = parse("3 mi run @ 8:00\n", pool(), None).expect("parses");
        match &w.sections[0].steps[0] {
            Step::Distance(d) => {
                assert_eq!(d.distance, Distance::miles(3));
                assert_eq!(d.interval, None);
                assert_eq!(d.notes.as_deref(), Some("@ 8:00"));
            }
            other => panic!("got {other:?}"),
        }
    }

    #[test]
    fn decimal_km_helpers() {
        assert_eq!(
            Distance::from_km_decimal("4.8"),
            Some(Distance::meters(4800))
        );
        assert_eq!(
            Distance::from_km_decimal("16.1"),
            Some(Distance::meters(16_100))
        );
        assert_eq!(Distance::from_km_decimal("5"), Some(Distance::meters(5000)));
        assert_eq!(Distance::from_km_decimal("0.0"), None);
        assert_eq!(Distance::from_km_decimal("abc"), None);
        assert_eq!(time_token("35:00"), Some(Seconds::minutes(35)));
        assert_eq!(time_token("1:02:03"), Some(Seconds::secs(3723)));
        assert_eq!(time_token("1:99"), None);
    }
}
