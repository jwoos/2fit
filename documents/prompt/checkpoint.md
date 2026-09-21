# Checkpoint (2026-09-21) — Turn 15: user decisions 1–4 done; NEXT: review

## Done — 4 commits (all green: 83 tests, clippy + fmt clean)

- **`bc0f071` sub-sport (#4)**: `SubSport::{LapSwim,OpenWater,Street,Trail,
  Track,IndoorRun,Road,IndoorBike,Generic,Other}` + `Workout.sub_sport`
  (serde-default, back-compat) + `effective_sub_sport()` (cross-sport falls
  back to per-sport default, never guesses) + encoder map (run Trail/TRACK/
  INDOOR_RUNNING, bike Trail→MOUNTAIN/Track→TRACK_CYCLING/INDOOR_CYCLING,
  swim OpenWater). `2fit-gen --sub-sport` presets (treadmill/indoor-run,
  indoor/mtb/track-cycling aliases). Test `sub_sport_overrides_and_fallbacks`.
- **`a986512` per-workout Higdon (#1)**: `plan_days()` (126 cells Novice 1)
  + `day_body()` (work day → 1 line, rest day → `—>W1 Mon Rest` = 0 steps)
  + `Higdon::fetch_days()` + `fetch --per-day [--out-dir DIR]
  [--skip-rest]` (stdout `=== title ===` bodies or `<slug>.txt` files) +
  `2fit-gen --name` (FIT `wkt_name`). Live: 86 day files, `w1-tue.txt` →
  111 B `.fit` with name encoded. Whole-plan `fetch` unchanged.
- **`8aecd29` target flags (#3)**: `Workout::{run_base,bike_ftp}` +
  `Thresholds` plumb (`parse_with_thresholds`, lib re-export) +
  `parse_target_text`/`target_word`/`trailing_pace_target`: `250W`→Power,
  `Z4`→HrZone, `95rpm`→Cadence direct (no flags); `110% FTP`→`ftp×pct`,
  `85% of 1mi pace`→`run_base/0.85` gated (stay notes without); beaten
  words kept in notes (last-wins, power beats cadence); ramps/named pace
  stay notes. `DistanceStep.target` (flatten prefers it over stroke map).
  `2fit-gen --run-base m:ss/km|/mi --ftp W`. Live smoke both paths.
- **`988ca00` bike source (#2)**: survey in `run-bike-notation.md` §3 —
  whatsonzwift Cloudflare-walled, TrainerRoad/Day walled, ergdb parked,
  bdcheung `.zwo` unlicensed/stale. Decision: **`.zwo` file import, no new
  scraper**. `parser::zwo` (Warmup/Cooldown/SteadyState/FreeRide→Timed,
  IntervalsT→Repeat, Ramp→midpoint+range notes, Power fractions×ftp,
  Cadence, sportType run) + `2fit-gen --zwo FILE [--ftp]` (rejects text
  flags). 2 attributed fixtures + 3 tests. Live: SST → 279 B `.fit`.

## Deferred (documented, not built)

- Named race pace (`5K pace` → needs pace map), ramp expansion (steady
  midpoint today), `FreeRide` open steps (target None — correct).
- `.zwo` `<textevent>` cues dropped (notes only carry labels today).
