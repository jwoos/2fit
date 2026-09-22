//! Zwift `.zwo` workout import → [`Workout`](fit_core::Workout).
//!
//! Why `.zwo` instead of scraping (evidence, 2026-09-21 — see
//! `documents/run-bike-notation.md` §3):
//!
//! - whatsonzwift.com is Cloudflare-walled; TrainerRoad/Day paywalled or
//!   walled; ergdb.org parked. No fetchable licensed bike corpus exists.
//! - `.zwo` is structured XML (durations + FTP fractions), mapping 1:1 to
//!   `Step::Timed` + `Target::Power` — no NLP needed. Element reference:
//!   h4l/zwift-workout-file-reference (built-in Zwift corpus analysis).
//! - Reference fixtures: `documents/bike-fixtures/zwo/` (bdcheung
//!   collection, unlicensed/stale — format reference + tests only,
//!   attributed here, never scraped as a `Site`).
//!
//! Element map (only what the IDL needs; the rest is ignored):
//! `Warmup`/`Cooldown`/`SteadyState`/`FreeRide` → one `TimedStep`
//! (label in notes); `IntervalsT` → `RepeatStep` (`Repeat` × [on, off]);
//! `Ramp` → per-minute `TimedStep`s via [`expand_ramp`](super::swimdojo::expand_ramp)
//! (interpolated watts; range in each step's notes);
//! `Power`/`PowerLow/High` (FTP fractions × `ftp`, default 250 W) →
//! `Target::Power`; `Cadence` → `Target::Cadence` (power wins when both
//! present — same last-wins as text targets). `sportType="run"` → [`Sport::Run`](fit_core::Sport), else Bike.
//! `FreeRide` with no power → target `None` (open).
//!
//! Coaching cues: `TextEvent`/`textevent` children (`message` +
//! `timeoffset`/`TimeOffset` seconds, `distoffset` meters on run files —
//! `mssage` is Zwift's own typo, read as fallback) append `@M:SS message`
//! (or `@Nm message`, or the bare message) to the containing step's notes.
//! Offsets map into expansions: a ramp cue lands in the minute-step
//! holding the offset; an intervals cue lands in the rep-phase inner step
//! (`on` vs `off` — visible every rep, since reps share inner steps).
//! Workout-level cues (direct `<workout>` children) attach to the step
//! holding the absolute offset. Empty messages are skipped.

use fit_core::{Intensity, Section, SectionLabel, Sport, Step, Target, TimedStep, Workout};

/// FTP watts assumed when the caller passes none (Zwift default athlete).
pub const DEFAULT_FTP: u32 = 250;

/// Error parsing a `.zwo` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "zwo: {}", self.0)
    }
}

impl std::error::Error for Error {}

/// Parse `.zwo` XML into a [`Workout`], resolving FTP fractions with `ftp`
/// (`None` = [`DEFAULT_FTP`]).
pub fn parse_zwo(xml: &str, ftp: Option<u32>) -> Result<Workout, Error> {
    let ftp = ftp.unwrap_or(DEFAULT_FTP);
    if ftp == 0 {
        return Err(Error("ftp must be > 0".into()));
    }
    let mut reader = quick_xml::Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut w = Workout::for_sport(Sport::Bike);
    w.name = None;
    let mut description: Option<String> = None;
    let mut section = Section {
        label: SectionLabel::Main,
        steps: Vec::new(),
        subtotal: None,
    };
    let mut buf = Vec::new();
    // Text-carrying elements (`TextEvent` children) need the event loop
    // to track nesting: a `Start` step element opens a frame; its `End`
    // closes it. Cues (`Empty` elements) attach to the innermost open
    // frame, or to the workout level when no frame is open. `handle_element`
    // pushes steps at `Start`, so the owner frame's base is the step's
    // own start (`elapsed` before adding its span).
    let mut elapsed: u32 = 0;
    let mut pending: Vec<PendingCue> = Vec::new();
    let mut stack: Vec<u32> = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                match name.as_str() {
                    // Text-carrying elements are read in the second pass
                    // (`tag_text`); workout elements go to `handle_element`.
                    "name" | "description" | "sportType" | "workout" | "workout_file" | "tags"
                    | "tag" => {}
                    "TextEvent" | "textevent" => {
                        if let Some(cue) = read_cue(&e) {
                            // Innermost open step frame, else workout level
                            // (base 0 — maps to the first step at attach).
                            pending.push(PendingCue {
                                base: stack.last().copied().unwrap_or(0),
                                message: cue.0,
                                offset: cue.1,
                            });
                        }
                    }
                    _ => {
                        let before = section.steps.len();
                        handle_element(&name, &e, &mut w, &mut description, &mut section, ftp)?;
                        elapsed += step_span(&section.steps[before..]);
                    }
                }
            }
            Ok(quick_xml::events::Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                match name.as_str() {
                    "name" | "description" | "sportType" | "workout" | "workout_file" | "tags"
                    | "tag" => {}
                    "TextEvent" | "textevent" => {
                        // Non-empty cue element (`<textevent>…</textevent>`):
                        // attributes still carry message/offset; treat the
                        // `Start` like the `Empty` form (inner text is
                        // ignored — Zwift cues are attribute-carried).
                        if let Some(cue) = read_cue(&e) {
                            pending.push(PendingCue {
                                base: stack.last().copied().unwrap_or(0),
                                message: cue.0,
                                offset: cue.1,
                            });
                        }
                    }
                    _ => {
                        // A step element opens a frame at its own start;
                        // nested cues belong to it.
                        stack.push(elapsed);
                        let before = section.steps.len();
                        handle_element(&name, &e, &mut w, &mut description, &mut section, ftp)?;
                        elapsed += step_span(&section.steps[before..]);
                    }
                }
            }
            Ok(quick_xml::events::Event::Text(e)) => {
                let _ = e;
            }
            Ok(quick_xml::events::Event::End(e)) => {
                // A step frame closes here — but only step elements open
                // frames (workout-level `End`s find an empty stack).
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if !matches!(
                    name.as_str(),
                    "name"
                        | "description"
                        | "sportType"
                        | "workout"
                        | "workout_file"
                        | "tags"
                        | "tag"
                        | "TextEvent"
                        | "textevent"
                ) {
                    stack.pop();
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(e) => return Err(Error(format!("xml: {e}"))),
            _ => {}
        }
        buf.clear();
    }
    // Text children (`<name>`, `<sportType>`) need a second pass: the
    // event loop above skips text. Re-scan cheaply for the two we need.
    w.name = tag_text(xml, "name");
    if let Some(sport) = tag_text(xml, "sportType")
        && sport.eq_ignore_ascii_case("run")
    {
        w.sport = Sport::Run;
        w.sub_sport = Workout::default_sub_sport(Sport::Run);
    }
    description = tag_text(xml, "description").filter(|d| !d.is_empty());
    w.description = description;
    attach_cues(&mut section.steps, &pending);
    if !section.steps.is_empty() {
        w.sections.push(section);
    }
    if w.sections.is_empty() {
        return Err(Error("no workout steps found".into()));
    }
    Ok(w)
}

/// A coaching cue read from the event loop, before its owner step's span
/// is fully known: `base` = absolute seconds at the containing step's
/// start, `offset` = `timeoffset` (or run `distoffset`, or the event's
/// `duration`) mapped into absolute seconds by [`attach_cues`].
struct PendingCue {
    /// Absolute seconds at the containing step's start.
    base: u32,
    /// Cue text (`message`, `mssage` fallback).
    message: String,
    /// Seconds (`timeoffset`/`TimeOffset`) or meters (`distoffset`) into
    /// the containing element; `None` = the element's start.
    offset: Option<CueOffset>,
}

/// A cue's position inside its containing element.
#[derive(Debug, Clone, Copy, PartialEq)]
enum CueOffset {
    /// Seconds into the containing element (`timeoffset`/`TimeOffset`).
    Time(u32),
    /// Meters into the containing element (run `distoffset` — unknown
    /// without the athlete's pace, so it attaches at the element start
    /// with `@Nm` kept verbatim).
    Dist(u32),
}

/// Read one `TextEvent`/`textevent` element into `(message, offset)`.
/// `None` = empty message (skipped — a cue with no text says nothing).
fn read_cue(e: &quick_xml::events::BytesStart<'_>) -> Option<(String, Option<CueOffset>)> {
    let attr = |key: &str| -> Option<String> {
        e.attributes().find_map(|a| {
            a.ok()
                .filter(|a| String::from_utf8_lossy(a.key.as_ref()).eq_ignore_ascii_case(key))
                .map(|a| String::from_utf8_lossy(&a.value).into_owned())
        })
    };
    let message = attr("message")
        .or_else(|| attr("mssage"))
        .map(|m| m.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|m| !m.is_empty())?;
    // `timeoffset` wins (the common coaching-cue clock); run files use
    // `distoffset` meters instead. Either may be float (`30.5`).
    let num = |v: Option<String>| -> Option<u32> {
        v?.trim().parse::<f64>().ok().map(|f| f.max(0.0) as u32)
    };
    let time = num(attr("timeoffset")).or_else(|| num(attr("TimeOffset")));
    let offset = match (time, num(attr("distoffset"))) {
        (Some(t), _) => Some(CueOffset::Time(t)),
        (None, Some(d)) => Some(CueOffset::Dist(d)),
        (None, None) => None,
    };
    Some((message, offset))
}

/// Total span of freshly pushed steps (so the event loop can track each
/// containing step's absolute start). Ramps are already expanded here —
/// the per-minute steps hold the cue's minute window.
fn step_span(steps: &[Step]) -> u32 {
    steps.iter().map(step_secs).sum()
}

/// Duration of one step: timed seconds, or repeats × inner (rest included).
fn step_secs(step: &Step) -> u32 {
    match step {
        Step::Timed(t) => t.duration.as_secs(),
        Step::Repeat(r) => {
            let inner: u32 = r.inner.iter().map(step_secs).sum();
            let rest = r.rest_between.map(|s| s.as_secs()).unwrap_or(0);
            r.count
                .saturating_mul(inner)
                .saturating_add(rest.saturating_mul(r.count.saturating_sub(1).max(1).min(r.count)))
        }
        Step::Rest { secs } => secs.as_secs(),
        // Distance/breakdown/recovery/technique carry no time (zero span —
        // `.zwo` never emits them, but the matcher must stay total).
        _ => 0,
    }
}

/// Attach pending cues to their owner steps: a cue belongs to the step
/// holding `base + offset` seconds (`distoffset` cues keep `@Nm` at the
/// containing element's start — meters need pace to map). Within an
/// owner, the offset maps into expansions: the ramp minute-step holding
/// the second, or the intervals rep-phase (`on`/`off`) holding it (cues
/// land on shared inner steps, so they repeat with the rep — the same
/// visibility Zwift gives them).
fn attach_cues(steps: &mut [Step], pending: &[PendingCue]) {
    // Absolute spans of the top-level steps.
    let mut spans: Vec<(u32, u32)> = Vec::with_capacity(steps.len());
    let mut cursor = 0u32;
    for s in steps.iter() {
        let len = step_secs(s);
        spans.push((cursor, cursor.saturating_add(len)));
        cursor = cursor.saturating_add(len);
    }
    for cue in pending {
        let abs = match cue.offset {
            Some(CueOffset::Time(t)) => cue.base.saturating_add(t),
            // At/below the base second: the containing element's start.
            Some(CueOffset::Dist(_)) | None => cue.base,
        };
        // Deepest step holding `abs` (or the last step when past the end —
        // a cue at exactly total duration belongs to the workout's tail).
        let idx = spans
            .iter()
            .position(|(a, b)| abs < *b || (*a == *b && abs == *a))
            .or(spans.len().checked_sub(1));
        let Some(i) = idx else { continue };
        let note = match cue.offset {
            Some(CueOffset::Time(t)) => {
                let into = abs.saturating_sub(spans[i].0);
                // At the step's own start, the bare message reads clean;
                // deeper offsets keep their clock.
                if into == 0 && t == 0 || cue.base == spans[i].0 && t == 0 {
                    cue.message.clone()
                } else {
                    format!("@{} {}", fmt_clock(into), cue.message)
                }
            }
            Some(CueOffset::Dist(d)) => format!("@{d}m {}", cue.message),
            None => cue.message.clone(),
        };
        push_cue(&mut steps[i], abs.saturating_sub(spans[i].0), &note);
    }
}

/// Append `note` to the deepest step holding `into` seconds in: ramp
/// minute-steps split by span; intervals rep-phases (`on`/`off`) by
/// cycle position; anything else takes the note whole.
fn push_cue(step: &mut Step, into: u32, note: &str) {
    match step {
        Step::Timed(t) => {
            t.notes = Some(match t.notes.take() {
                Some(prev) => format!("{prev} — {note}"),
                None => note.to_owned(),
            });
        }
        Step::Repeat(r) => {
            let cycle: u32 = r.inner.iter().map(step_secs).sum();
            let rest = r.rest_between.map(|s| s.as_secs()).unwrap_or(0);
            let stride = cycle.saturating_add(rest);
            let at = if stride == 0 { 0 } else { into % stride };
            let mut cursor = 0u32;
            for inner in r.inner.iter_mut() {
                let len = step_secs(inner);
                if at < cursor.saturating_add(len) || len == 0 {
                    // Re-clock the note to the phase (`off` at 60 s in a
                    // 60/60 cycle shows `@0:30` 30 s in, not the absolute
                    // `@1:30` — the inner step repeats, so absolute clocks
                    // lie on later reps).
                    let phase_at = at.saturating_sub(cursor);
                    let note = reclock_note(note, phase_at);
                    push_cue(inner, phase_at, &note);
                    return;
                }
                cursor = cursor.saturating_add(len);
            }
            // In the `rest_between` gap (or empty inner): note the whole rep.
            r.notes = Some(match r.notes.take() {
                Some(prev) => format!("{prev} — {note}"),
                None => note.to_owned(),
            });
        }
        Step::Distance(d) => {
            d.notes = Some(match d.notes.take() {
                Some(prev) => format!("{prev} — {note}"),
                None => note.to_owned(),
            });
        }
        Step::Breakdown(b) => {
            b.notes = Some(match b.notes.take() {
                Some(prev) => format!("{prev} — {note}"),
                None => note.to_owned(),
            });
        }
        Step::Recovery(r) => {
            r.notes = Some(match r.notes.take() {
                Some(prev) => format!("{prev} — {note}"),
                None => note.to_owned(),
            });
        }
        // Rests take no notes; technique steps never appear in `.zwo`
        // (but the matcher stays total over the non-exhaustive `Step`).
        _ => {}
    }
}

/// `M:SS` clock for cue offsets (`90` → `1:30`).
fn fmt_clock(secs: u32) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

/// Re-clock an `@M:SS message` note to a rep-phase offset: the absolute
/// clock (`@1:30`) becomes the phase clock (`@0:30` 30 s into `off`).
/// Bare messages (no clock) pass through.
fn reclock_note(note: &str, phase_at: u32) -> String {
    let Some((_, rest)) = note.split_once(' ') else {
        return note.to_owned();
    };
    if !note.starts_with('@') {
        return note.to_owned();
    }
    format!("@{} {rest}", fmt_clock(phase_at))
}

fn handle_element(
    name: &str,
    e: &quick_xml::events::BytesStart<'_>,
    w: &mut Workout,
    _description: &mut Option<String>,
    section: &mut Section,
    ftp: u32,
) -> Result<(), Error> {
    let attr = |key: &str| -> Option<String> {
        e.attributes().find_map(|a| {
            a.ok()
                .filter(|a| String::from_utf8_lossy(a.key.as_ref()).eq_ignore_ascii_case(key))
                .map(|a| String::from_utf8_lossy(&a.value).into_owned())
        })
    };
    let duration = || -> Result<u32, Error> {
        attr("Duration")
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|v| *v > 0)
            .ok_or_else(|| Error(format!("<{name}> needs Duration > 0")))
    };
    let power_of = |v: Option<String>| -> Option<u32> {
        v.and_then(|v| v.parse::<f64>().ok())
            .map(|f| (f * ftp as f64).round() as u32)
            .filter(|v| *v > 0)
    };
    let cadence_of = || -> Option<Target> {
        attr("Cadence")
            .and_then(|v| v.parse::<u8>().ok())
            .filter(|v| *v > 0)
            .map(Target::Cadence)
    };
    let timed = |secs: u32, power: Option<u32>, cadence: Option<Target>, label: String| -> Step {
        Step::Timed(TimedStep {
            duration: fit_core::Seconds::secs(secs),
            target: power.map(Target::Power).or(cadence),
            intensity: Intensity::Active,
            notes: Some(label),
        })
    };
    match name {
        "Warmup" | "Cooldown" | "SteadyState" | "FreeRide" => {
            let secs = duration()?;
            let power = power_of(attr("Power")).or_else(|| {
                let (lo, hi) = (power_of(attr("PowerLow")), power_of(attr("PowerHigh")));
                match (lo, hi) {
                    (Some(l), Some(h)) => Some((l + h) / 2),
                    (Some(l), None) => Some(l),
                    (None, Some(h)) => Some(h),
                    (None, None) => None,
                }
            });
            let label = if power.is_some() || name == "FreeRide" {
                name.to_ascii_lowercase()
            } else {
                format!("{name} (open)")
            };
            // Warmup/Cooldown note their range; target holds the midpoint
            // (ramps below expand instead — this arm is steady-state only).
            let notes = if name == "Warmup" || name == "Cooldown" {
                let (lo, hi) = (power_of(attr("PowerLow")), power_of(attr("PowerHigh")));
                match (lo, hi, power) {
                    (Some(l), Some(h), Some(_)) if l != h => {
                        format!("{} {l}-{h}W", name.to_ascii_lowercase())
                    }
                    _ => label,
                }
            } else {
                label
            };
            section.steps.push(timed(secs, power, cadence_of(), notes));
        }
        "Ramp" => {
            let secs = duration()?;
            let (lo, hi) = (power_of(attr("PowerLow")), power_of(attr("PowerHigh")));
            match (lo, hi) {
                (Some(l), Some(h)) => {
                    let ramp = super::swimdojo::PowerRamp {
                        duration: fit_core::Seconds::secs(secs),
                        lo: l.min(h),
                        hi: l.max(h),
                        label: format!("ramp {l}-{h}W"),
                    };
                    section
                        .steps
                        .extend(super::swimdojo::expand_ramp(&ramp, String::new(), None));
                }
                // Degenerate ramp (one/no bound) = steady step at the bound.
                _ => {
                    let power = power_of(attr("Power")).or(lo).or(hi);
                    let label = if power.is_some() {
                        "ramp".to_owned()
                    } else {
                        "Ramp (open)".to_owned()
                    };
                    section.steps.push(timed(secs, power, cadence_of(), label));
                }
            }
        }
        "IntervalsT" => {
            let repeat: u32 = attr("Repeat").and_then(|v| v.parse().ok()).unwrap_or(1);
            let (on_secs, off_secs) = (
                attr("OnDuration")
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(0),
                attr("OffDuration")
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(0),
            );
            if on_secs == 0 || repeat == 0 {
                return Err(Error(
                    "<IntervalsT> needs OnDuration > 0 and Repeat > 0".into(),
                ));
            }
            let (on_power, off_power) = (power_of(attr("OnPower")), power_of(attr("OffPower")));
            let mut inner = vec![Step::Timed(TimedStep {
                duration: fit_core::Seconds::secs(on_secs),
                target: on_power.map(Target::Power).or_else(cadence_of),
                intensity: Intensity::Interval,
                notes: Some("on".into()),
            })];
            if off_secs > 0 {
                inner.push(Step::Timed(TimedStep {
                    duration: fit_core::Seconds::secs(off_secs),
                    target: off_power.map(Target::Power),
                    intensity: Intensity::Recovery,
                    notes: Some("off".into()),
                }));
            }
            section.steps.push(Step::Repeat(fit_core::RepeatStep {
                count: repeat,
                rest_between: None,
                inner,
                notes: None,
            }));
        }
        _ => {}
    }
    let _ = w;
    Ok(())
}

/// Inner text of the first `<tag>…</tag>`.
fn tag_text(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let start = xml.find(&open)?;
    let inner = xml[start..].find('>')? + start + 1;
    let end = xml[inner..].find(&format!("</{tag}>"))? + inner;
    let text = xml[inner..end].trim();
    if text.is_empty() || text.contains('<') {
        return None;
    }
    Some(
        html_unescape(text)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn html_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SST: &str = include_str!("../../../documents/bike-fixtures/zwo/sst-traditional.zwo");
    const RECOVERY: &str = include_str!("../../../documents/bike-fixtures/zwo/active-recovery.zwo");

    #[test]
    fn sst_maps_to_timed_power() {
        let w = parse_zwo(SST, Some(250)).unwrap();
        assert_eq!(w.sport, Sport::Bike);
        assert_eq!(w.name.as_deref(), Some("Traditional SST"));
        let flat = w.flat_steps();
        assert_eq!(flat.len(), 3);
        assert_eq!(flat[0].time, Some(fit_core::Seconds::secs(600)));
        // Warmup 0.25–0.75 FTP → midpoint 0.5 × 250 = 125 W.
        assert_eq!(flat[0].target, Some(Target::Power(125)));
        // SteadyState 0.90 × 250 = 225 W.
        assert_eq!(flat[1].target, Some(Target::Power(225)));
        assert_eq!(flat[1].time, Some(fit_core::Seconds::secs(2700)));
    }

    #[test]
    fn recovery_defaults_ftp() {
        let w = parse_zwo(RECOVERY, None).unwrap();
        assert_eq!(w.name.as_deref(), Some("Active Recovery"));
        let flat = w.flat_steps();
        assert!(!flat.is_empty());
        // 0.65 × 250 = 162.5 → 163 W (round, not truncate).
        assert_eq!(flat[0].target, Some(Target::Power(163)));
    }

    #[test]
    fn ramp_expands_per_minute() {
        let xml = r#"<workout_file><name>Ramp Test</name><workout>
            <Ramp Duration="180" PowerLow="0.4" PowerHigh="0.8"/>
        </workout></workout_file>"#;
        let w = parse_zwo(xml, Some(250)).unwrap();
        let flat = w.flat_steps();
        // 3min 100→200 W: 100/150/200, range in each step's notes.
        assert_eq!(flat.len(), 3);
        assert_eq!(flat[0].target, Some(Target::Power(100)));
        assert_eq!(flat[1].target, Some(Target::Power(150)));
        assert_eq!(flat[2].target, Some(Target::Power(200)));
        for s in &flat {
            let notes = s.notes.as_deref().unwrap_or("");
            assert!(notes.contains("100-200W"), "{notes:?}");
        }
        // Degenerate ramp (single bound) stays one steady step.
        let xml = r#"<workout_file><name>Half Ramp</name><workout>
            <Ramp Duration="120" PowerLow="0.5"/>
        </workout></workout_file>"#;
        let w = parse_zwo(xml, Some(250)).unwrap();
        let flat = w.flat_steps();
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].target, Some(Target::Power(125)));
    }

    #[test]
    fn textevents_attach_as_cues() {
        // Step-child cue at offset 0: bare message on the owning step.
        let xml = r#"<workout_file><name>Cues</name><workout>
            <SteadyState Duration="300" Power="0.6"><textevent timeoffset="0" message="Stay smooth"/>
            </SteadyState>
        </workout></workout_file>"#;
        let w = parse_zwo(xml, Some(250)).unwrap();
        let flat = w.flat_steps();
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].target, Some(Target::Power(150)));
        let notes = flat[0].notes.as_deref().unwrap_or("");
        assert!(notes.contains("steadystate"), "{notes:?}");
        assert!(notes.contains("Stay smooth"), "{notes:?}");
        // Mid-step offset keeps its clock (`@1:30`).
        let xml = r#"<workout_file><name>Cues</name><workout>
            <SteadyState Duration="300" Power="0.6"><textevent timeoffset="90" message="Halfway"/>
            </SteadyState>
        </workout></workout_file>"#;
        let w = parse_zwo(xml, Some(250)).unwrap();
        let notes = w.flat_steps()[0].notes.as_deref().unwrap_or("").to_owned();
        assert!(notes.contains("@1:30 Halfway"), "{notes:?}");
        // Intervals cue lands in the rep-phase inner step (`off` here —
        // visible every rep, since reps share inner steps).
        let xml = r#"<workout_file><name>Cues</name><workout>
            <IntervalsT Repeat="3" OnDuration="60" OffDuration="60" OnPower="0.8" OffPower="0.5"><textevent timeoffset="90" message="Recover"/>
            </IntervalsT>
        </workout></workout_file>"#;
        let w = parse_zwo(xml, Some(250)).unwrap();
        let flat = w.flat_steps();
        assert_eq!(flat.len(), 6);
        assert!(!flat[0].notes.as_deref().unwrap_or("").contains("Recover"));
        // `off` holds the cue (at its 30 s cycle offset → `@0:30`);
        // every rep shows it (shared inner step — same as Zwift).
        for f in [&flat[1], &flat[3], &flat[5]] {
            assert!(
                f.notes.as_deref().unwrap_or("").contains("@0:30 Recover"),
                "{flat:?}"
            );
        }
        // Ramp cue maps into the minute-step holding the offset.
        let xml = r#"<workout_file><name>Cues</name><workout>
            <Ramp Duration="180" PowerLow="0.4" PowerHigh="0.8"><textevent timeoffset="90" message="Push"/>
            </Ramp>
        </workout></workout_file>"#;
        let w = parse_zwo(xml, Some(250)).unwrap();
        let flat = w.flat_steps();
        assert_eq!(flat.len(), 3);
        let (n0, n1, n2) = (
            flat[0].notes.as_deref().unwrap_or(""),
            flat[1].notes.as_deref().unwrap_or(""),
            flat[2].notes.as_deref().unwrap_or(""),
        );
        assert!(!n0.contains("Push"), "{n0:?}");
        assert!(n1.contains("@0:30 Push"), "{n1:?}");
        assert!(!n2.contains("Push"), "{n2:?}");
        // `TextEvent` (capital) + `mssage` typo + run `distoffset` forms.
        let xml = r#"<workout_file><name>Cues</name><sportType>run</sportType><workout>
            <SteadyState Duration="600"><TextEvent TimeOffset="30" message="Relax"/></SteadyState>
            <SteadyState Duration="600"><textevent distoffset="400" mssage="400m mark"/></SteadyState>
        </workout></workout_file>"#;
        let w = parse_zwo(xml, Some(250)).unwrap();
        assert_eq!(w.sport, Sport::Run);
        let flat = w.flat_steps();
        let notes0 = flat[0].notes.as_deref().unwrap_or("");
        assert!(notes0.contains("@0:30 Relax"), "{notes0:?}");
        let notes1 = flat[1].notes.as_deref().unwrap_or("");
        assert!(notes1.contains("@400m 400m mark"), "{notes1:?}");
        // Empty messages skipped; cueless files parse cue-free.
        let xml = r#"<workout_file><name>Cues</name><workout>
            <SteadyState Duration="60" Power="0.5"><textevent timeoffset="10" message="  "/></SteadyState>
        </workout></workout_file>"#;
        let w = parse_zwo(xml, Some(250)).unwrap();
        let notes = w.flat_steps()[0].notes.clone().unwrap_or_default();
        assert!(!notes.contains('@'), "{notes:?}");
        let w = parse_zwo(SST, Some(250)).unwrap();
        assert_eq!(w.flat_steps()[0].notes.as_deref(), Some("warmup 63-188W"));
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse_zwo("<workout_file>", Some(250)).is_err());
        assert!(parse_zwo(SST, Some(0)).is_err());
        assert!(
            parse_zwo(
                "<workout_file><workout></workout></workout_file>",
                Some(250)
            )
            .is_err()
        );
    }
}
