# Plan

Goal: two Rust binaries in one Cargo workspace sharing a core "IDL" + a rustyfit-based .fit encoder. Decision: use **rustyfit v0.10.2** (see Decisions).

## Architecture (Cargo workspace)

- `core/` (lib `fit_core`): parse-independent domain types = the IDL (Workout/Step/Stroke/Interval/Pool, units, time).
- `generator/` (lib + bin `2fit-gen`):
  - `parser/`: format-specific parsers → core. First: `swimdojo`.
  - `fit/`: core → .fit bytes via rustyfit.
  - CLI: `--file <path>` or stdin (`-`/no arg) → `.fit` (file or stdout). Also usable as a library (`parse_str`, `to_fit`).
- `scraper/` (lib + bin `2fit-scrape`): swimdojo scrape + filters (distance/level/stroke/tag/author/pagination); emit normalized notation text; later: infer a site's format → schema (JSON) for the parser.

## Decisions (data-backed)

- FIT crate = **rustyfit v0.10.2**. Evidence: it *encodes* (Decoder+Encoder, FIT Protocol v2, `Encoder`/`EncoderBuilder`/`StreamEncoder`), and its `mesgdef` structs carry the swim-relevant fields we need — `Workout:{pool_length, pool_length_unit, wkt_name, wkt_description, sport}`, `WorkoutStep:{duration_type, target_type, intensity, wkt_step_name, notes}`, `WktStepDuration::DISTANCE`, `WktStepTarget::SWIM_STROKE`. `fit-sdk-rust` also encodes but is younger (0.2.x, crate named `fit`).
- Standard format is the IDL: a Workout = metadata + an ordered list of steps; a step is single-distance / repeat / breakdown / rest / recovery. FIT has **no repeat primitive** — repeats are expanded to concrete steps (`num_valid_steps` counts them).
- Rest/intervals (swimdojo `@`) are first-class in the IDL, then emitted as FIT `TIME` steps.

## Standard format (IDL)

Canonical spec + evidence: **documents/idl.md** (implemented in `core/src/lib.rs`, 16 unit tests).
Deviations from the old draft, with reasons, are noted there (notably: `Interval` split into `IntervalSpec::Fixed | Base{offset}` + `Workout.base100`, since bases are swimmer data, never written in the workout).

## .fit mapping (core → rustyfit)

Verified against rustyfit 0.10.2 source: **documents/rustyfit.md** (constants: sport=swim 5, sub_sport=lap_swimming 17, WktStepDuration TIME 0/DISTANCE 1, WktStepTarget SWIM_STROKE 11, pool_length scale×100, Intensity 0..6). Open items (swim-stroke target codes, `capabilities` bits, in-memory `Write+Seek` writer) live there too.
