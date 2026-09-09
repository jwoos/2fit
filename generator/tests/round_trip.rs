//! Parse → encode → decode round-trip on a real swimdojo fixture.

use std::io::Cursor;

use embedded_io_adapters::std::FromStd;
use fit_core::Pool;
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
