# Checkpoint (2026-09-21) — Turn 17: named race pace; NEXT: user review

## Done — 1 commit (green: 85 tests, clippy + fmt clean)

- **`3d4ca88` named race pace** (closes Phase 3 deferral): core
  `Workout.race_paces: BTreeMap<String, Pace>` (serde-default,
  back-compat) + `pace_key()` (lowercase alphanumeric) +
  `Workout::race_pace()` lookup; `Thresholds.race_paces` plumbed
  (`parse_with_thresholds` seeds `w.race_paces`); `target_word` tries
  the map after `%`/clock (`@ marathon pace`, bare `@ 5k`);
  `trailing_pace_target` takes the longest resolving suffix window
  (`8 x 400 5K pace`, `1600 m Med Pace`); `2fit-gen --race-pace
  NAME=PACE` (repeatable, `pace_key`-normalized, `--zwo`-rejected).
  Unknown names stay notes. Tests: `named_race_paces_resolve` +
  `race_pace_flags` (85 total). Live: 228 B `.fit`, SPEED 3448.
  Docs: `idl.md` Workout row, `run-bike-notation.md` § Named race paces.

## Deferred (remaining)

- Ramp expansion (steady midpoint today), `FreeRide` open steps
  (target None — correct), `.zwo` `<textevent>` cues dropped.
