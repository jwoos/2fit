# Checkpoint (2026-09-21) — Turn 12: Phase 2 CLI + per-sport round-trips done; NEXT: Phase 3 parser/schemas

## Done — commit `9c86e03` (all green: 63 tests, clippy + fmt clean)

- **`2fit-gen --sport swim|run|bike`** (default `swim`): swim honors `--pool`
  (default `25yd`); run/bike reject `--pool` (`Workout.pool = None`) and parse
  bare numbers in a throwaway 25-yd pool (unit source only — a Phase 3 run
  parser supplies explicit km/mi). `resolve_pool()` + `pool_resolution` test.
- **Per-sport round-trips** (`generator/tests/round_trip.rs`, mirrors goblin):
  `run_workout_round_trip` (3 mi DISTANCE 482_803 + 2×1:00@5:00/km TIME/SPEED,
  RUNNING/STREET, pool absent `u16::MAX`); `bike_workout_round_trip`
  (2×20:00@250W TIME/POWER, CYCLING/ROAD).
- Live smoke: `--help` shows sport values; `--sport run --pool 25yd` errors.

## NEXT: Phase 3 — Parser + schemas (planned, no code)

- Unit suffixes `km`/`mi`/`k` in `parse_step`; `schema::{run(),bike()}`;
  `documents/run-fixtures/`, `documents/bike-fixtures/`; `infer` learns
  pace/power markers (verify before adding fields). Interval-style shapes
  (`8 x 400 @ 5k pace`, `2 x 20:00 @ 250W`) still to-confirm vs a real site.
- Deferred from Phase 1: `run_base`/`bike_ftp` (no parser produces them yet).
