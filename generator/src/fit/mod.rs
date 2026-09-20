//! Encodes `fit_core` workouts into .fit protocol files (via `rustyfit`).
//!
//! Mapping (core → FIT profile; verified field facts in
//! `documents/rustyfit.md`):
//!
//! - one `FileId` (type `workout`) + one `Workout` message (sport swim /
//!   lap swim, name, description, pool length);
//! - one `WorkoutStep` per [`Workout::flat_steps`] entry (repeats and IMs
//!   already expanded, technique drills already dropped);
//! - a distance step → `duration_type = DISTANCE` with `duration_value` in
//!   meters × 100 (FIT's distance unit); a rest step → `TIME` + seconds;
//! - a known stroke → `target_type = SWIM_STROKE` + `target_value`; an
//!   unspecified stroke (`None`, the swimdojo freestyle default) and
//!   `Stroke::Any` (no FIT code exists for "any non-free") omit the target;
//! - `Workout.capabilities` is left at its default (skipped: the field is
//!   UINT32Z whose invalid value is 0), so no capability bits are claimed.

use std::io::Cursor;

use embedded_io_adapters::std::FromStd;
use fit_core::{FlatStep, Intensity, Stroke, Unit, Workout};
use rustyfit::{
    profile::{mesgdef, typedef},
    proto::{Message, FIT},
    Encoder,
};

/// Error from encoding a [`Workout`] into .fit bytes (std io backend).
pub type Error = rustyfit::EncoderError<std::io::Error>;

/// Encode `workout` into a complete, standalone .fit file (bytes).
pub fn to_fit(workout: &Workout) -> Result<Vec<u8>, Error> {
    let mut fit = FIT {
        messages: build_messages(workout),
        ..Default::default()
    };
    let mut buf = Vec::<u8>::new();
    let mut enc = Encoder::new();
    enc.encode(FromStd::new(Cursor::new(&mut buf)), &mut fit)?;
    Ok(buf)
}

/// The FIT messages of `workout`, in protocol order (`FileId`, `Workout`,
/// then one `WorkoutStep` per flat step).
fn build_messages(workout: &Workout) -> Vec<Message> {
    let flat = workout.flat_steps();
    let mut out = Vec::with_capacity(2 + flat.len());
    out.push(Message::from(file_id()));
    out.push(Message::from(workout_message(workout, flat.len())));
    out.extend(flat.iter().map(|s| Message::from(step_message(s))));
    out
}

fn file_id() -> mesgdef::FileId {
    let mut m = mesgdef::FileId::new();
    m.r#type = typedef::File::WORKOUT;
    m.manufacturer = typedef::Manufacturer::DEVELOPMENT;
    m.product_name = "2fit".to_owned();
    m
}

fn workout_message(workout: &Workout, num_steps: usize) -> mesgdef::Workout {
    let mut m = mesgdef::Workout::new();
    m.sport = typedef::Sport::SWIMMING;
    m.sub_sport = typedef::SubSport::LAP_SWIMMING;
    m.num_valid_steps = num_steps as u16;
    m.wkt_name = fit_str(&workout.name.clone().unwrap_or_default());
    m.wkt_description = fit_str(&workout.description.clone().unwrap_or_default());
    m.pool_length = meters_x100(workout.pool.length, workout.pool.unit) as u16;
    m.pool_length_unit = match workout.pool.unit {
        Unit::Meters => typedef::DisplayMeasure::METRIC,
        Unit::Yards => typedef::DisplayMeasure::STATUTE,
        // A future unit has no known display measure; metric is FIT's
        // native one.
        _ => typedef::DisplayMeasure::METRIC,
    };
    m
}

fn step_message(step: &FlatStep) -> mesgdef::WorkoutStep {
    let mut m = mesgdef::WorkoutStep::new();
    m.wkt_step_name = fit_str(&step.name.clone().unwrap_or_default());
    m.notes = fit_str(&step.notes.clone().unwrap_or_default());
    m.intensity = intensity(step.intensity);
    if let Some(t) = step.time {
        m.duration_type = typedef::WktStepDuration::TIME;
        m.duration_value = t.as_secs();
    } else if let Some(d) = step.distance {
        m.duration_type = typedef::WktStepDuration::DISTANCE;
        m.duration_value = meters_x100(d.value, d.unit);
        if let Some((target_type, target_value)) = stroke_target(step.stroke) {
            m.target_type = target_type;
            m.target_value = target_value;
        }
    }
    m
}

fn intensity(i: Intensity) -> typedef::Intensity {
    match i {
        Intensity::Active => typedef::Intensity::ACTIVE,
        Intensity::Rest => typedef::Intensity::REST,
        Intensity::Warmup => typedef::Intensity::WARMUP,
        Intensity::Cooldown => typedef::Intensity::COOLDOWN,
        Intensity::Recovery => typedef::Intensity::RECOVERY,
        Intensity::Interval => typedef::Intensity::INTERVAL,
        Intensity::Other => typedef::Intensity::OTHER,
        // A future core intensity has no FIT code; other is the catch-all.
        _ => typedef::Intensity::OTHER,
    }
}

/// The FIT target for `stroke`: `(target_type, target_value)` when the
/// stroke has a `SWIM_STROKE` code, else `None` (target omitted).
fn stroke_target(stroke: Option<Stroke>) -> Option<(typedef::WktStepTarget, u32)> {
    let value = match stroke? {
        Stroke::Free => typedef::SwimStroke::FREESTYLE,
        Stroke::Back => typedef::SwimStroke::BACKSTROKE,
        Stroke::Breast => typedef::SwimStroke::BREASTSTROKE,
        Stroke::Fly => typedef::SwimStroke::BUTTERFLY,
        Stroke::IM => typedef::SwimStroke::IM,
        // "Any non-free" has no single FIT code; the target is omitted so
        // the step is simply swum without a stroke designation.
        Stroke::Any => return None,
        // A future stroke has no FIT code; omit the target rather than guess.
        _ => return None,
    };
    Some((typedef::WktStepTarget::SWIM_STROKE, u32::from(value.0)))
}

/// Truncate `s` to fit a FIT `string` field (max 255 bytes incl. NUL).
/// Cuts at a char boundary and never splits UTF-8.
fn fit_str(s: &str) -> String {
    const MAX: usize = 254;
    if s.len() <= MAX {
        return s.to_owned();
    }
    let mut end = MAX;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_owned()
}

/// FIT's distance unit for `value` pool units: meters × 100, with
/// 1 yd = 0.9144 m, 1 mi = 1609.344 m (integer round). Saturates at `u32::MAX`.
fn meters_x100(value: u32, unit: Unit) -> u32 {
    match unit {
        Unit::Meters => value.saturating_mul(100),
        Unit::Kilometers => value.saturating_mul(100_000),
        Unit::Yards => ((value as u64 * 9_144 + 50) / 100)
            .try_into()
            .unwrap_or(u32::MAX),
        Unit::Miles => ((value as u64 * 16_093_440 + 50) / 100)
            .try_into()
            .unwrap_or(u32::MAX),
        // A future unit is assumed to be meters (FIT's native distance
        // unit) rather than mis-converted through the yard factor.
        _ => value.saturating_mul(100),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fit_core::{
        Distance, DistanceStep, IntervalSpec, Pool, Seconds, Section, SectionLabel, Step,
    };
    use rustyfit::Decoder;

    fn sample() -> Workout {
        let mut w = Workout::new(Pool::yards25());
        w.name = Some("Goblin Shark".into());
        w.description = Some("turn 6 encoder test".into());
        w.sections.push(Section {
            label: SectionLabel::Main,
            steps: vec![
                Step::Distance(DistanceStep {
                    distance: Distance::yards(100),
                    stroke: Some(Stroke::Back),
                    interval: Some(IntervalSpec::Fixed(Seconds::minutes(2))),
                    intensity: Intensity::Active,
                    notes: Some("high elbow".into()),
                }),
                Step::Rest {
                    secs: Seconds::secs(30),
                },
                Step::Distance(DistanceStep {
                    distance: Distance::yards(50),
                    stroke: None,
                    interval: None,
                    intensity: Intensity::Interval,
                    notes: None,
                }),
            ],
            subtotal: None,
        });
        w
    }

    fn decode(bytes: &[u8]) -> FIT {
        let mut dec = Decoder::new();
        dec.decode(FromStd::new(Cursor::new(bytes)))
            .unwrap()
            .expect("a complete FIT file")
    }

    #[test]
    fn fit_magic_and_header() {
        let bytes = to_fit(&sample()).unwrap();
        assert_eq!(&bytes[8..12], b".FIT");
    }

    #[test]
    fn round_trip_mapping() {
        let fit = decode(&to_fit(&sample()).unwrap());
        assert_eq!(fit.messages.len(), 5); // FileId, Workout, 3 steps

        let fid = mesgdef::FileId::from(&fit.messages[0]);
        assert_eq!(fid.r#type, typedef::File::WORKOUT);
        assert_eq!(fid.manufacturer, typedef::Manufacturer::DEVELOPMENT);
        assert_eq!(fid.product_name, "2fit");

        let w = mesgdef::Workout::from(&fit.messages[1]);
        assert_eq!(w.sport, typedef::Sport::SWIMMING);
        assert_eq!(w.sub_sport, typedef::SubSport::LAP_SWIMMING);
        assert_eq!(w.num_valid_steps, 3);
        assert_eq!(w.wkt_name, "Goblin Shark");
        assert_eq!(w.wkt_description, "turn 6 encoder test");
        // 25 yd = 22.86 m, stored meters × 100; STATUTE display unit so a
        // watch renders 25 yd, not 22.86.
        assert_eq!(w.pool_length, 2286);
        assert_eq!(w.pool_length_unit, typedef::DisplayMeasure::STATUTE);

        let s0 = mesgdef::WorkoutStep::from(&fit.messages[2]);
        assert_eq!(s0.duration_type, typedef::WktStepDuration::DISTANCE);
        assert_eq!(s0.duration_value, 9144); // 100 yd = 91.44 m
        assert_eq!(s0.target_type, typedef::WktStepTarget::SWIM_STROKE);
        assert_eq!(s0.target_value, 1); // backstroke
        assert_eq!(s0.intensity, typedef::Intensity::ACTIVE);
        assert_eq!(s0.wkt_step_name, "100 yd back @ 2:00");
        assert_eq!(s0.notes, "high elbow");

        let s1 = mesgdef::WorkoutStep::from(&fit.messages[3]);
        assert_eq!(s1.duration_type, typedef::WktStepDuration::TIME);
        assert_eq!(s1.duration_value, 30);
        assert_eq!(s1.intensity, typedef::Intensity::REST);
        assert!(s1.target_type.0 == u8::MAX); // omitted, round-trips as absent

        let s2 = mesgdef::WorkoutStep::from(&fit.messages[4]);
        assert_eq!(s2.duration_type, typedef::WktStepDuration::DISTANCE);
        assert_eq!(s2.duration_value, 4572); // 50 yd = 45.72 m
        assert_eq!(s2.intensity, typedef::Intensity::INTERVAL);
        assert_eq!(s2.wkt_step_name, "50 yd");
        assert!(s2.target_type.0 == u8::MAX); // stroke None → target omitted
    }

    #[test]
    fn meters_pool_encoding() {
        let mut w = Workout::new(Pool::meters50());
        w.sections.push(Section {
            label: SectionLabel::WarmUp,
            steps: vec![Step::Distance(DistanceStep {
                distance: Distance::meters(400),
                stroke: Some(Stroke::Fly),
                interval: None,
                intensity: Intensity::default(),
                notes: None,
            })],
            subtotal: None,
        });
        let fit = decode(&to_fit(&w).unwrap());
        let wm = mesgdef::Workout::from(&fit.messages[1]);
        assert_eq!(wm.pool_length, 5000);
        assert_eq!(wm.pool_length_unit, typedef::DisplayMeasure::METRIC);
        let s = mesgdef::WorkoutStep::from(&fit.messages[2]);
        assert_eq!(s.duration_value, 40000); // 400 m × 100
        assert_eq!(s.target_value, 3); // butterfly
    }

    #[test]
    fn stroke_targets_and_omissions() {
        fn one_step(stroke: Option<Stroke>) -> Workout {
            let mut w = Workout::new(Pool::yards25());
            w.sections.push(Section {
                label: SectionLabel::None,
                steps: vec![Step::Distance(DistanceStep {
                    distance: Distance::yards(50),
                    stroke,
                    interval: None,
                    intensity: Intensity::default(),
                    notes: None,
                })],
                subtotal: None,
            });
            w
        }
        let expected = |stroke: Option<Stroke>| match stroke {
            Some(Stroke::Free) => Some(0),
            Some(Stroke::Back) => Some(1),
            Some(Stroke::Breast) => Some(2),
            Some(Stroke::Fly) => Some(3),
            // 50 IM is not a standard IM distance, so it is not broken down
            // and the defensive IM target (6) is emitted.
            Some(Stroke::IM) => Some(6),
            // Any (no FIT code) and future strokes → target omitted.
            _ => None,
        };
        for stroke in [
            Some(Stroke::Free),
            Some(Stroke::Back),
            Some(Stroke::Breast),
            Some(Stroke::Fly),
            Some(Stroke::IM),
            Some(Stroke::Any),
            None,
        ] {
            let fit = decode(&to_fit(&one_step(stroke)).unwrap());
            let s = mesgdef::WorkoutStep::from(&fit.messages[2]);
            let got = (s.target_type.0 != u8::MAX).then_some(s.target_value);
            assert_eq!(got, expected(stroke), "stroke {stroke:?}");
        }
    }

    #[test]
    fn run_units_encode_as_meters() {
        for (dist, want) in [
            (Distance::kilometers(5), 500_000), // 5 km × 100
            (Distance::miles(3), 482_803),      // 3 × 1609.344 m, rounded
        ] {
            let mut w = Workout::new(Pool::yards25());
            w.sections.push(Section {
                label: SectionLabel::None,
                steps: vec![Step::Distance(DistanceStep {
                    distance: dist,
                    stroke: None,
                    interval: None,
                    intensity: Intensity::default(),
                    notes: None,
                })],
                subtotal: None,
            });
            let fit = decode(&to_fit(&w).unwrap());
            let s = mesgdef::WorkoutStep::from(&fit.messages[2]);
            assert_eq!(s.duration_value, want, "distance {dist}");
        }
    }

    #[test]
    fn long_strings_truncate_to_fit_limit() {
        let mut w = sample();
        match &mut w.sections[0].steps[0] {
            Step::Distance(d) => d.notes = Some("n".repeat(300)),
            other => panic!("got {other:?}"),
        }
        let fit = decode(&to_fit(&w).unwrap());
        let s = mesgdef::WorkoutStep::from(&fit.messages[2]);
        assert_eq!(s.notes.len(), 254);
        assert_eq!(fit_str(&"é".repeat(200)).len(), 254);
    }
}
