//! Parse → encode → decode round-trips: swim fixture + run/bike core builds.

use std::io::Cursor;

use embedded_io_adapters::std::FromStd;
use fit_core::{
    Distance, DistanceStep, Intensity, Pool, Seconds, Section, SectionLabel, Sport, Step, Target,
    TimedStep,
};
use fit_generator::{fit, parser::swimdojo};
use rustyfit::{
    Decoder,
    profile::{mesgdef, typedef},
    proto::FIT,
};

const GOBLIN: &str = include_str!("../../documents/swimdojo-fixtures/goblin-shark.txt");

fn decode(bytes: &[u8]) -> FIT {
    let mut dec = Decoder::new();
    dec.decode(FromStd::new(Cursor::new(bytes)))
        .unwrap()
        .expect("a complete FIT file")
}

#[test]
fn goblin_shark_round_trip() {
    let workout = swimdojo::parse(GOBLIN, Pool::yards25(), None).expect("parses");
    let flat = workout.flat_steps();
    assert!(!flat.is_empty());

    let fit = decode(&fit::to_fit(&workout).unwrap());
    assert_eq!(fit.messages.len(), 2 + flat.len());

    let w = mesgdef::Workout::from(&fit.messages[1]);
    assert_eq!(w.num_valid_steps, flat.len() as u16);
    assert_eq!(w.pool_length, 2286); // 25 yd = 22.86 m, stored m × 100
    assert_eq!(w.pool_length_unit, typedef::DisplayMeasure::STATUTE);

    // First step: `500 swim` → freestyle default, no stroke target.
    let s0 = mesgdef::WorkoutStep::from(&fit.messages[2]);
    assert_eq!(s0.duration_type, typedef::WktStepDuration::DISTANCE);
    assert_eq!(s0.duration_value, 45_720); // 500 yd = 457.20 m
    assert_eq!(s0.wkt_step_name, "500 yd");
    assert!(s0.target_type.0 == u8::MAX);

    // `800 IM` is not a breakdown distance (only 100/200/300 IMs split),
    // so it stays one step with the defensive IM target and keeps its
    // base interval in the name.
    let im = flat
        .iter()
        .position(|s| s.name.as_deref() == Some("800 yd IM @ b+120"))
        .expect("an 800 IM step");
    let s = mesgdef::WorkoutStep::from(&fit.messages[2 + im]);
    assert_eq!(s.duration_type, typedef::WktStepDuration::DISTANCE);
    assert_eq!(s.target_type, typedef::WktStepTarget::SWIM_STROKE);
    assert_eq!(s.target_value, 6); // IM
}

/// Run workout round-trip: `3 mi` distance + `2 x 1:00 @ 5:00/km` timed,
/// encoded as RUNNING/STREET with no pool and SPEED targets.
#[test]
fn run_workout_round_trip() {
    let mut workout = fit_core::Workout::for_sport(Sport::Run);
    workout.name = Some("Easy 3".into());
    workout.sections.push(Section {
        label: SectionLabel::Main,
        steps: vec![
            Step::Distance(DistanceStep {
                distance: Distance::miles(3),
                stroke: None,
                interval: None,
                target: None,
                intensity: Intensity::Active,
                notes: None,
            }),
            Step::Repeat(fit_core::RepeatStep {
                count: 2,
                rest_between: None,
                inner: vec![Step::Timed(TimedStep {
                    duration: Seconds::minutes(1),
                    target: Some(Target::Pace(fit_core::Pace::from_secs_per_km(300))),
                    intensity: Intensity::Interval,
                    notes: None,
                })],
                notes: None,
            }),
        ],
        subtotal: None,
    });
    let flat = workout.flat_steps();
    assert_eq!(flat.len(), 3);

    let fit = decode(&fit::to_fit(&workout).unwrap());
    assert_eq!(fit.messages.len(), 2 + flat.len());

    let w = mesgdef::Workout::from(&fit.messages[1]);
    assert_eq!(w.sport, typedef::Sport::RUNNING);
    assert_eq!(w.sub_sport, typedef::SubSport::STREET);
    assert_eq!(w.wkt_name, "Easy 3");
    assert_eq!(w.num_valid_steps, 3);
    assert_eq!(w.pool_length, u16::MAX); // absent, not zero

    let s0 = mesgdef::WorkoutStep::from(&fit.messages[2]);
    assert_eq!(s0.duration_type, typedef::WktStepDuration::DISTANCE);
    assert_eq!(s0.duration_value, 482_803); // 3 mi = 4828.03 m
    assert!(s0.target_type.0 == u8::MAX); // no target → omitted

    let s1 = mesgdef::WorkoutStep::from(&fit.messages[3]);
    assert_eq!(s1.duration_type, typedef::WktStepDuration::TIME);
    assert_eq!(s1.duration_value, 60);
    assert_eq!(s1.target_type, typedef::WktStepTarget::SPEED);
    assert_eq!(s1.target_value, 1_000_000 / 300);
    assert_eq!(s1.intensity, typedef::Intensity::INTERVAL);
}

/// Bike workout round-trip: `2 x 20:00 @ 250W`, CYCLING/ROAD, POWER targets.
#[test]
fn bike_workout_round_trip() {
    let mut workout = fit_core::Workout::for_sport(Sport::Bike);
    workout.sections.push(Section {
        label: SectionLabel::Main,
        steps: vec![Step::Repeat(fit_core::RepeatStep {
            count: 2,
            rest_between: None,
            inner: vec![Step::Timed(TimedStep {
                duration: Seconds::minutes(20),
                target: Some(Target::Power(250)),
                intensity: Intensity::Active,
                notes: None,
            })],
            notes: None,
        })],
        subtotal: None,
    });

    let fit = decode(&fit::to_fit(&workout).unwrap());
    assert_eq!(fit.messages.len(), 4); // FileId, Workout, 2 steps

    let w = mesgdef::Workout::from(&fit.messages[1]);
    assert_eq!(w.sport, typedef::Sport::CYCLING);
    assert_eq!(w.sub_sport, typedef::SubSport::ROAD);
    assert_eq!(w.num_valid_steps, 2);
    assert_eq!(w.pool_length, u16::MAX);

    for msg in &fit.messages[2..] {
        let s = mesgdef::WorkoutStep::from(msg);
        assert_eq!(s.duration_type, typedef::WktStepDuration::TIME);
        assert_eq!(s.duration_value, 1200);
        assert_eq!(s.target_type, typedef::WktStepTarget::POWER);
        assert_eq!(s.target_value, 250);
    }
}
