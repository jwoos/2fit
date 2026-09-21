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
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Empty(e)) | Ok(quick_xml::events::Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                match name.as_str() {
                    // Text-carrying elements are read in the second pass
                    // (`tag_text`); workout elements go to `handle_element`.
                    "name" | "description" | "sportType" | "workout" | "workout_file" | "tags"
                    | "tag" | "textevent" => {}
                    _ => handle_element(&name, &e, &mut w, &mut description, &mut section, ftp)?,
                }
            }
            Ok(quick_xml::events::Event::Text(e)) => {
                let _ = e;
            }
            Ok(quick_xml::events::Event::End(_)) => {}
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
    if !section.steps.is_empty() {
        w.sections.push(section);
    }
    if w.sections.is_empty() {
        return Err(Error("no workout steps found".into()));
    }
    Ok(w)
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
