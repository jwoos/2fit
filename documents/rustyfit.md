# rustyfit 0.10.2 — encoder notes

Verified against the actual crate source in the cargo registry
(`~/.cargo/registry/src/*/rustyfit-0.10.2/`), 2026-08-31. Crate:
rustyfit 0.10.2 (latest on crates.io as of this date; released 2026-08-11,
edition 2024, MSRV `rust-version = 1.93`).

## Why rustyfit (decision, data-backed)

- It **encodes** (we need encode, not decode): `Encoder::encode` +
  `StreamEncoder`.
- Its `profile::mesgdef` structs carry exactly the swim fields we need:
  `Workout { sport, sub_sport, pool_length, pool_length_unit, wkt_name,
  wkt_description, num_valid_steps, ... }`,
  `WorkoutStep { duration_type, duration_value, target_type, target_value,
  intensity, wkt_step_name, notes, ... }`.
- Alternative `fit-sdk-rust` (crate `fit`) also encodes but is younger
  (0.2.x); rustyfit is newer, actively published (4 releases in ~2 months),
  `no_std`-first.

## Encoding pattern (from crate docs + source)

```rust
use embedded_io::{Seek, Write};               // traits from embedded-io 0.7
use rustyfit::{
    Encoder,
    profile::{mesgdef, typedef},
    proto::FIT,
};

let mut fit = FIT {
    messages: vec![
        Message::from(file_id),   // mesgdef::FileId
        Message::from(workout),   // mesgdef::Workout
        Message::from(step),      // mesgdef::WorkoutStep (one per flat step)
        // ...
    ],
    ..Default::default()
};
let mut writer = MemWriter::new();            // our Vec<u8> Write+Seek
let mut enc = Encoder::new();
enc.encode(&mut writer, &mut fit)?;           // W: Write + Seek, fit: &mut FIT
```

Key signatures (source):

- `Encoder::encode<W: Write + Seek>(&mut self, writer: W, fit: &mut FIT) -> Result<(), Error<W::Error>>`
  — writer by value, `FIT` by **mut** ref (encoder rewrites `file_header.data_size`).
- `FIT { file_header: FileHeader, messages: Vec<Message>, crc: u16 }`, `Default`
  leaves header at 0s — `encode` fills data size. 14-byte header (with CRC) is
  what `encode` reserves first (`writer.write_all(&[0u8; 14])`).
- `Message::from(mesgdef)` exists for every mesgdef (field-by-field From impls);
  invalid default values (`u8::MAX` etc.) are **skipped** on encode
  (`count_valid_fields` gates each field), so leave unused fields at
  `new()` defaults.
- `Encoder::builder()` options: `endianness`, `protocol_version`,
  `header_option` — defaults (LittleEndian, V2, 14-byte header) are what we want.

## Our writer problem

`Encode` needs `Write + Seek` (§ embedded_io::Write/Seek, NOT std). For a `.fit`
file output we wrap `File`; for stdout (no seek) we buffer: encode into
`Vec<u8>` in memory (workout files are tiny — a handful of WorkoutStep
messages), then `stdout().write_all(&bytes)`.
embedded-io 0.7 provides `impl Write for Vec<u8>` (no Seek), so we may need a
trivial `Cursor<Vec<u8>>` adapter (both `Seek`+`Write`) or to seek a `Vec`.
Verify during impl whether a std adapter (`embedded-io-std`?) is needed.

## Swimmer-relevant constants (from `profile/typedef/`)

| Item | Value | Source |
| --- | --- | --- |
| `Sport::SWIMMING` | `5` | sport.rs |
| `SubSport::LAP_SWIMMING` | `17` (the "pool" swim) | sub_sport.rs (no plain `POOL`; 126 = pool_triathlon) |
| `File::WORKOUT` | `5` | file.rs |
| `WktStepDuration::TIME` / `DISTANCE` | `0` / `1` | wkt_step_duration.rs |
| `WktStepTarget::SWIM_STROKE` | `11` (others: speed=0, hr=1, ...) | wkt_step_target.rs |
| `Intensity::ACTIVE/REST/WARMUP/COOLDOWN/RECOVERY/INTERVAL/OTHER` | `0..6` | intensity.rs |
| `DisplayMeasure::METRIC/STATUTE/NAUTICAL` | `0/1/2` | display_measure.rs |
| `Workout.pool_length` | uint16, **scale 100, unit m** | workout.rs (`set_pool_length_scaled`) |
| `WorkoutStep.duration_value` | uint32 (m × 100 for DISTANCE per Profile; secs for TIME) | workout_step.rs |
| `FileId` fields | TYPE=0, MANUFACTURER=1, PRODUCT=2, SERIAL_NUMBER=3, TIME_CREATED=4, NUMBER=5, PRODUCT_NAME=8 | file_id.rs |

## Open question: `target_value` for swim stroke

For `target_type = SWIM_STROKE(11)`, the FIT profile defines the stroke values
(free=0, breast=1, back=2, fly=3, IM=4) — confirm the exact enum (may be a
`typedef::SwimStroke` or inline in `WktStepTarget`) before encoding. Grep
`swim` in `profile/typedef/`. `Any`/non-freestyle has no single FIT code —
plan: map `Any` → omit `target_value` (or pick a representative); decide when
implementing the encoder.

## `Workout.capabilities`

`WorkoutCapabilities` (uint32z) is a bitfield of which step duration/target
types are used. If we only emit DISTANCE+SWIM_STROKE and TIME steps, set the
corresponding bits (or leave default — verify decoder tolerance). Decide in
encoder impl.
