# Checkpoint (2026-09-20) — Turn 10: multisport Phase 0 done; NEXT: Phase 1 Core IDL

## Done — Phase 0 survey (no code)

- **`documents/run-bike-notation.md`** (new): Hal Higdon Novice 1 marathon fetched
  live — `<N> mi/km run` cells, bare long-run distances (unit from grid toggle),
  `Rest`/`Cross` day markers (map to "no step", not `Step::Rest`). Midweek prose
  carries effort ("easy", "MP-30..90s/mi") outside the cell text ⇒ base run format
  has no per-step target; pace/power/HR targets live only in interval formats
  (`8 x 400 @ 5k pace`, `2 x 20:00 @ 250W` — still to-confirm in Phase 3).
- **FIT facts re-verified in rustyfit 0.10.2 source** (checkpoint's turn-9 claim
  now evidenced): `Sport` RUNNING=1/CYCLING=2; `SubSport` GENERIC=0/STREET=2/
  TRAIL=3/TRACK=4/ROAD=7/MOUNTAIN=8/INDOOR_CYCLING=6/INDOOR_RUNNING=45;
  targets SPEED=0/HEART_RATE=1/CADENCE=3/POWER=4; durations TIME=0/DISTANCE=1.
  Bonus finds: `WorkoutStep` already has `custom_target_value_low/high` +
  secondary-target fields (pace/power ranges = mapping, no new message);
  encoder wildcard arms over `#[non_exhaustive]` core enums ⇒ additive core
  changes compile.

## NEXT: Phase 1 — Core IDL (planned, no code yet)

Per turn-9 plan (unchanged): `Sport { Swim, Run, Bike }` + `Workout.sport`
(default `Swim`); `Workout.pool → Option<Pool>`; `Unit += Kilometers, Miles`;
`Step::Timed`; `stroke → target: Option<Target>`; `run_base`/`bike_ftp`.
One commit per type; tests per commit. Run first (SPEED = smallest encoder
delta), then bike (POWER/CADENCE).
