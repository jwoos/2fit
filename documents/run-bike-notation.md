# Run/Bike notation survey (Phase 0, multisport)

Evidence for the run/bike IDL shapes (`Sport`, `Step::Timed`, `Target`,
`Unit += Kilometers/Miles`). Counterpart to `swimdojo-grammar.txt` (swim)
and `myswimpro.md` (second swim site).

## Source 1 (fetched 2026-09-20): Hal Higdon Novice 1 Marathon

URL: https://www.halhigdon.com/training-programs/marathon-training/novice-1-marathon/
An 18-week marathon plan as a Mon..Sun grid; each cell is one workout line.
Observed line shapes (verbatim):

- `<N> mi run` — e.g. `3 mi run`, `10 mi run` (midweek easy / Sorta-Long Runs)
- `<N> km run` — same cells in metric view, e.g. `4.8 km run`, `16.1 km run`
- bare distance — long-run cells, e.g. `6`, `20` (unit from grid toggle: miles or km)
- `Rest`, `Cross`, `Half Marathon`, `Marathon` (non-run placeholders)

Evidence details:

- Units are venue/display-level: the same cell reads `3 mi run` / `4.8 km run`
  (3 mi = 4.8 km), `20` / `32.2`. So run distances need `mi`/`km` units in the
  IDL, with conversion at parse/display time — same role as swim's yd/m.
- Midweek lines carry an effort qualifier only in prose ("comparatively easy
  pace", "30 to 90 seconds or more per mile slower than marathon pace"), not
  in the cell text. So the base plan format has **no per-step target**; pace
  targets appear only in interval-style formats (see §2).
- `Rest` / `Cross` cells are whole-day markers, not timed rest steps: they map
  to "no step" (like swim Technique), not to `Step::Rest`.

## Source 2 (proposed, NOT yet fetched): interval-style run/bike lines

These shapes are assumed from the checkpoint plan and must be confirmed
against a real site in Phase 3 before parser work:

- `8 x 400 @ 5k pace` — repeat + distance + named pace target
- `2 x 20:00 @ 250W` — repeat + duration + power target
- `95 rpm` — cadence target; `min/km` — pace unit; HR zones; %FTP

Do NOT build schema tables for these yet; Phase 3 verifies first.

## Confirmed IDL shapes

- `Sport { Swim, Run, Bike }` + `Workout.sport` (default `Swim`).
  `Workout.pool → Option<Pool>` (`None` for run/bike; encoder omits pool).
- `Unit += Kilometers, Miles` (Higdon §1: `mi`/`km` are the run units).
- `Step::Timed { duration, target, … }`: run/bike work is often time-based
  (`20:00`, long-run minutes). `FlatStep` already models `distance XOR time`,
  so the encoder delta is small.
- `stroke → target: Option<Target>` with
  `Target { Swim(Stroke), Pace, Power, HrZone, Cadence }` (swim mapping
  preserved; Higdon base format uses no target, intervals use Pace/Power/HR).
- `base100` + `run_base: Option<Pace>` + `bike_ftp: Option<u32>`
  (parallels swim's swimmer-owned base; TrainingPeaks threshold/zone model:
  one threshold per sport, zones derived — see references).

## FIT facts (verified in rustyfit 0.10.2 source, 2026-09-20)

- `src/profile/typedef/sport.rs`: `RUNNING = Sport(1)`, `CYCLING = Sport(2)`
  (`SWIMMING = 5` unchanged).
- `src/profile/typedef/sub_sport.rs`: `GENERIC = 0`, `STREET = 2`, `TRAIL = 3`,
  `TRACK = 4`, `ROAD = 7`, `MOUNTAIN = 8`, `INDOOR_CYCLING = 6`,
  `INDOOR_RUNNING = 45`, `GRAVEL_CYCLING = 46`.
- `src/profile/typedef/wkt_step_target.rs`: `SPEED = 0`, `HEART_RATE = 1`,
  `CADENCE = 3`, `POWER = 4` (`SWIM_STROKE = 11` unchanged).
- `src/profile/typedef/wkt_step_duration.rs`: `TIME = 0`, `DISTANCE = 1`
  (plus HR/power/repeat variants for later).
- `src/profile/mesgdef/workout_step.rs`: `WorkoutStep` already carries
  `custom_target_value_low/high` + `secondary_target_type/value` — pace/power
  ranges need no new message, just target mapping.
- Encoder compat: `fit::intensity` and `stroke_target` end in wildcard arms
  over `#[non_exhaustive]` core enums, so additive core changes compile.
