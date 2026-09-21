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

## Source 3 (surveyed 2026-09-21, NOT scraped): bike workout sources

User asked for a second bike source beyond hand excerpts. Findings:

- **whatsonzwift.com**: richest free Zwift text (`10min from 24 to 60% FTP`,
  `7x 1min @ 65% FTP, 1min @ 50% FTP`, `2min @ 60rpm, 70% FTP`,
  `4x 4min @ 100rpm, 75% FTP, 3min @ 85rpm, 50% FTP` — Recovery collection,
  fetched live). BUT direct `curl` hits Cloudflare challenge (`Just a
  moment...`, 5378 B); the `markdrayton/wozzwo` repo exists precisely to
  parse these pages, implying HTML scraping is brittle. No `.zwo`
  downloads, no API. **Verdict: hostile to scraping; skip.**
- **TrainerRoad / TrainerDay**: paywalled / Cloudflare-walled. Skip.
- **ergdb.org**: domain parked (casino review, 2026). Dead. Skip.
- **`bdcheung/zwift_workouts` (GitHub, 39 `.zwo` files, ★25, no license,
  stale 2019)**: raw XML fetch works (`SST.zwo`: `Warmup/SteadyState/
  Cooldown` with `PowerLow/High` FTP fractions). `.zwo` is a *better*
  bike source than text scraping: structured durations + FTP fractions
  map 1:1 to `Step::Timed` + `Target::Power` (with `--ftp`) — no NLP.
  But: no license + stale + one author's collection. **Verdict: format
  reference + offline test corpus (attribute), not a scraped `Site`.**
- **Recommendation**: bike ingestion = `.zwo` file import
  (`2fit-gen --zwo file.zwo --ftp 250` → IDL → `.fit`), using the
  `zwift-workout-file-reference` schema (h4l repo) for the element map
  (`Warmup/Cooldown/SteadyState/Intervals/FreeRide` + `Power`,
  `PowerLow/High`, `Cadence`). No new scraper until a licensed,
  fetchable corpus appears. Hand Zwift excerpts stay the text fixtures.

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

## Named race paces (2026-09-21; closes the Phase 3 deferral)

`--race-pace NAME=PACE` (repeatable; value = `--run-base` shape
`m:ss/km|/mi`) fills `Workout.race_paces` (keys via `pace_key`:
lowercase alphanumeric, so `5K`/`5k`/`5-K` match). Resolution —
trailing qualifier takes the longest resolving suffix window
(`1600 m Med Pace` → `med`+`pace`; `8 x 400 5K pace` → `5k`), `@`
targets try the map after `%`/clock forms (`20min @ marathon pace`,
bare `@ 5k`). Unknown names stay notes (back-compat; no-flag behavior
unchanged). Live: `8 x 400 5K pace --race-pace 5k=4:50/km` → 228 B
`.fit` with SPEED targets (3448 = 1e6/290).

## Ramp expansion (2026-09-21; closes the steady-midpoint deferral)

Ramps expand to per-minute `TimedStep`s (user choice): `ceil(dur/60s)`
steps, full 60 s minutes first with the remainder last (`90s` → 60+30),
watts interpolate linearly `lo + (hi−lo)×i/(n−1)` (1-step ramp = midpoint).
Each step's notes carry the full range (`from 30 to 70% FTP`).

- Text: `10min from 75 to 70W` → 10×1:00 @ 75…70 W; `5min from 30 to
  70% FTP` → 5 steps 75→175 W (`--ftp 250`); `20min @ from 30 to 70%
  FTP` (the `@` side) expands the same. Shared-unit endpoints: bare `30`
  beside `%` in FTP context = 30% FTP (not 30 W); explicit `W` always
  watts. Ungated `%FTP` (no `--ftp`) stays one notes step (back-compat).
  Live: `10min from 75 to 70W` → 467 B `.fit` (10 steps).
- `.zwo`: `<Ramp Duration PowerLow/High>` → same expansion
  (`label: ramp {lo}-{hi}W` in notes); degenerate (one/no bound) stays
  one steady step. `Warmup`/`Cooldown` ranges keep the old midpoint +
  range-notes behavior (steady by format, not ramps).
- Drive-by fix: `split_count` rewritten anchored-at-start
  (digits-then-separator only) — `10min` no longer parses as count
  `10` × `min`, which the ramp work exposed (`10min from…` → 1 step,
  not 10). `5x60m`/`4 x 100`/`4×50` unchanged; `3x through:` still a
  repeat-block marker.

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
