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
// Resolved pattern (used by generator/src/fit/mod.rs, verified working):
use embedded_io_adapters::std::FromStd;       // embedded-io-adapters 0.7, feature "std"
use rustyfit::{
    Encoder,
    profile::{mesgdef, typedef},
    proto::{FIT, Message},
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
let mut buf = Vec::<u8>::new();
let mut enc = Encoder::new();
enc.encode(FromStd::new(std::io::Cursor::new(&mut buf)), &mut fit)?;
// bytes: `buf` (Cursor<&mut Vec<u8>>; FromStd adapts embedded-io → std)
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

## Our writer problem — RESOLVED

`encode` needs `Write + Seek` (embedded-io 0.7 traits, NOT std). No custom
writer needed: **`embedded_io_adapters::std::FromStd`** (crate
`embedded-io-adapters` 0.7, feature `std`) wraps any std type —
`FromStd::new(Cursor::new(&mut buf))` gives a by-value `Write + Seek` over
`&mut Vec<u8>`; the bytes afterwards are in `buf`. For file output the CLI
just writes the `Vec<u8>` (no need to encode to a `File` directly).
Dep added to workspace (`embedded-io-adapters 0.7.0` feature `std`) — the
same dep rustyfit itself uses in its tests.

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

## `target_value` for swim stroke — RESOLVED

`typedef::SwimStroke(pub u8)` (swim_stroke.rs), the `target_value` when
`target_type = SWIM_STROKE(11)`. Verified values (note: differ from the old
FIT-profile guess above — the crate source is authoritative):

| const | value | const | value |
| --- | --- | --- | --- |
| FREESTYLE | 0 | DRILL | 4 |
| BACKSTROKE | 1 | MIXED | 5 |
| BREASTSTROKE | 2 | IM | 6 |
| BUTTERFLY | 3 | RIMO | 8 |

(no 7; IM_BY_ROUND exists in the profile between IM and RIMO.)
Mapping (decided, encoder committed 2026-09-08): Free→0, Back→1,
Breast→2, Fly→3, IM→6 (defensive: standard 100/200/300 IMs are already
broken down by `flat_steps`), `Any`/`None`/future strokes → **omit target
entirely** (`target_type` left at `u8::MAX`, skipped on encode).

## `Workout.capabilities` — RESOLVED

Left 0. The field is **UINT32Z whose invalid value is 0**, so a 0 is skipped
on encode (no capabilities claimed); round-trip decoder tolerance verified in
`fit::tests::round_trip_mapping`. Revisit only if a target type beyond
SWIM_STROKE/TIME+DISTANCE is ever emitted (bit 0 = "custom" per profile).
