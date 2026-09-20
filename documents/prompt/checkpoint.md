# Checkpoint (2026-09-20) — Turn 11: Phase 1 Core IDL done; NEXT: Phase 2 encoder per sport / CLI

## Done — 3 atomic commits (all green: 60 tests, clippy + fmt clean)

- **`d15fc11` Unit Kilometers/Miles**: `Unit += Kilometers, Miles` (`km`/`mi`
  abbreviations, `Distance::{kilometers,miles}`); encoder `meters_x100` handles
  both (km×100_000, mi×160_934.4 round; 3 mi → 482_803). Round-trip test
  `run_units_encode_as_meters`.
- **`bf529c9` Sport + optional pool**: `Sport { Swim, Run, Bike }` (default Swim),
  `Workout.sport` (`#[serde(default)]`, back-compat JSON), `Workout.pool: Option<Pool>`
  (`Workout::new` sets `Some`; `Workout::for_sport` sets `None`),
  `total_distance() -> Option<Distance>` (`None` pool-less). Encoder maps
  swim→SWIMMING/LAP_SWIMMING, run→RUNNING/STREET, bike→CYCLING/ROAD; pool-less
  leaves `pool_length(_unit)` at invalid sentinels (encoder omits them —
  verified `pool_length_scaled() -> None` on `u16::MAX` + serializer skips).
  Test `sport_sub_sport_per_sport` asserts codes + omission. Deferred:
  `run_base`/`bike_ftp` (no parser produces pace/power yet — Phase 3).
- **`0713617` Target + Timed**: `Pace(secs/km)` + `Target::{Swim,Pace,Power,HrZone,Cadence}`;
  `Step::Timed { duration, target, intensity, notes }` (distance_value 0,
  display `20:00 @ 250W`); `FlatStep.target` (swim flattening sets both
  `stroke` and `target`; timed sets `target` only); encoder `target()` maps
  Pace→SPEED m/s×1000, Power→POWER, HrZone→HEART_RATE, Cadence→CADENCE,
  wins over legacy `stroke`. Tests: `timed_step_flattens_with_target` (core),
  `timed_targets_round_trip` (encoder: 5 steps incl. 2×20:00@250W repeat).
- Swim behavior unchanged: `Workout::dist` still panics only for pool-less
  (swim parsers always set pool); legacy `stroke` path intact.

## NEXT: Phase 2 — Encoder per sport + CLI (planned, no code)

- Encoder is done for run/bike (this turn); remaining: round-trip tests per
  sport mirroring the goblin test (swim covered, bike timed covered, run
  distance+timed still thin) + `2fit-gen --sport run|bike|swim` (pool
  swim-only, `--base` generalized to pace/FTP later).
- Then Phase 3 (parser + schemas): unit suffixes `km`/`mi`/`k`, `schema::{run(),bike()}`,
  fixtures, `infer` pace/power markers (verify before adding fields).
