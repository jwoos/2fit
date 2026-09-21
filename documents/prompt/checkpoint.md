# Checkpoint (2026-09-21) — Turn 18: ramp expansion; NEXT: user review

## Done — 1 commit (green: 87 tests, clippy + fmt clean)

- **`beb67e0` ramp expansion** (user chose per-minute): `PowerRamp`
  (`duration/lo/hi/label`) + `expand_ramp()` (ceil/60 steps, 60 s first
  + remainder last, linear watts, range in each step's notes) +
  `parse_power_ramp()`/`ramp_tail()`/`ramp_endpoint_watts()` (shared-unit
  endpoints: bare `30` beside `%` in FTP context = 30% FTP; explicit `W`
  always watts; ungated `%FTP` stays notes). Text: `10min from 75 to
  70W` → 10 steps, `5min from 30 to 70% FTP` → 75→175 W, `20min @ from
  30 to 70% FTP` expands on the `@` side. `.zwo` `<Ramp>` → same
  expansion (`ramp {lo}-{hi}W` notes); degenerate = steady step.
  Drive-by: `split_count` anchored-at-start (`10min` no longer count
  `10`). Tests: `power_ramps_expand_per_minute` + `ramp_expands_per_minute`
  (zwo) + updated `threshold_targets_resolve` (87 total). Live: 467 B
  / 328 B `.fit` smokes. Docs: `run-bike-notation.md` § Ramp expansion.

## Deferred (remaining)

- `FreeRide` open steps (target None — correct), `.zwo` `<textevent>`
  cues dropped.
