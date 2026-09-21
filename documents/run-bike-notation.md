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

## Source 2 (fetched 2026-09-21): interval-style run/bike lines

Confirmed against real sites (both fetched live; excerpts in fixtures):

- Higdon Intermediate 10K grid (https://www.halhigdon.com/training-programs/10k-training/intermediate-10k/):
  `8 x 400 5K pace`, `9 x 400 5K pace`, `10 x 400 5K pace` (Wed speedwork —
  bare meters + named `5K pace` qualifier, no `@`); `35 min tempo run`,
  `40 min tempo run`, `60 min cross` (leading durations); `3.5 mi run`,
  `5.9 km run` (decimal miles/km). Program prose: 400s "at about the pace
  you would run in a 5K race", tempo runs "buildup … to near race pace".
- Zwift via whatsonzwift.com (https://whatsonzwift.com/workouts/ftp-tests/,
  `Zwift 101: Running` plan page): bike `5min free ride`,
  `1min @ 85rpm, 100W` … `1min @ 85rpm, 250W` (spaceless durations, `@`
  cadence+watts — NOT a swim interval); `10min from 75 to 70W`,
  `5min from 30 to 70% FTP`, `20min free ride target 110% FTP` (ramps +
  `%FTP`); run `400 m Walk`, `800 m Jog At Easy Pace`, `1600 m Med Pace`,
  `4x 200 m @ 110% of 1mi pace, 200 m @ 70% of 1mi pace` (spaceless counts,
  `% of 1mi pace`, comma-joined reps).

Phase 3 decision (evidence-backed): `%FTP` / `% of 1mi pace` / named
`5K pace` / watts+rpm stay **notes**, not `Target` — resolving them needs
athlete thresholds (`run_base`/`bike_ftp`, deferred since Phase 1), and
Higdon's base cells carry no target at all. No new schema fields were
needed: `higdon_run()` / `zwift()` differ from swimdojo only in
vocabulary tables (freestyle/rest/recovery words); `infer` learns the
run/bike filler (`run`, `tempo`, `fast`, `ride`, `walk`, `jog`, `cross`)
into `freestyle_words` with swim-only tables honestly gapped.

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
