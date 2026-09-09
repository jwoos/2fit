# Checkpoint (2026-09-09) — Second site done; NEXT: multisport (run+bike)

## Done — commit `344bf72` (second site: myswimpro WP scraper)

- **`Myswimpro` site** (`scraper/src/site/myswimpro.rs` + `Swimdojo::blocks/clean_block` now document-order + shared): WP REST API listing (`categories=15`, `search=`, `NaiveDateTime` wall-time dates) + slug-lookup fetch (`content.rendered`, raw-HTML fallback) + normalizer (section headings incl. `Kick & Drill`/`IM Set (x2)`/`The Workout (…)`, `×→ x`, dryland dropped, `Round N` → `—>`). 4 live-fetched fixtures in `documents/myswimpro-fixtures/`; 4 offline tests (posts, body, steps, 3 page fixtures).
- **Parser gaps closed** (all test-covered): multi-separator `split_count` (` x `/`×`/spaceless `5x60m`), unit-suffixed distances (`60m`/`50s`), `schema::myswimpro()` constructor + fixture tests, `infer` learns repeat-suffixed labels/`×` tokens/`freestyle`/`drill(s)`. Real bug fixed: rest marker is literal `rest`, not `rest_words[0]` (inference order put `freestyle` first → 300 m swim became a 300 s rest).
- **End-to-end**: `--site myswimpro list/fetch` live-verified; `fetch …usrpt… | 2fit-gen → 1840 B .fit`. Inferred MSP schema: honest gaps (`total_prefix`, `repeat_word`, `clock_prefix`, `base_marker` — MSP uses none); parses to identical structure, notes-level `.fit` diffs only.
- `documents/myswimpro.md` (new): mechanics, divergences, out-of-scope, parser changes, inference report.
- Workspace: 55 tests (18 core + 22 generator + 15 scraper), clippy + fmt clean. No new deps (serde_json/chrono already vendored).

## NEXT: multisport — run + bike alongside swim (planned, no code)

What stays the same: expansion model (`flat_steps`), `Intensity`, notes/`fit_str`, `FileId`, `Site` trait, schema-driven parsing (all sport-agnostic). FIT facts verified in rustyfit 0.10.2 source: `Sport` RUNNING=1, CYCLING=2; `SubSport` GENERIC=0, STREET=2, TRAIL=3, TRACK=4, INDOOR_CYCLING=6, INDOOR_RUNNING=45; targets SPEED=0, HEART_RATE=1, CADENCE=3, POWER=4; durations TIME=0, DISTANCE=1. Encoder wildcard arms on `#[non_exhaustive]` enums mean additive core changes compile.

- **Phase 0 — Survey notation (no code)**: run/bike written formats first (`8 x 400 @ 5k pace`, `2 x 20:00 @ 250W`, `95 rpm`, `min/km`). One evidence page in `documents/`, like the swimdojo grammar. Confirms `Target`/`Timed` shapes.
- **Phase 1 — Core IDL**: `Sport { Swim, Run, Bike }` + `Workout.sport` (default `Swim`, back-compat); `Workout.pool → Option<Pool>` (`None` for run/bike, encoder omits `pool_length`); `Unit += Kilometers, Miles`; new `Step::Timed { duration, target, … }` (run/bike time-based work; `FlatStep` already `distance XOR time` so encoder barely changes); `stroke → target: Option<Target>` with `Target { Swim(Stroke), Pace, Power, HrZone, Cadence }` (swim mapping preserved); `base100` + `run_base: Option<Pace>` + `bike_ftp: Option<u32>`. One commit per type; tests per commit.
- **Phase 2 — Encoder per sport**: swim → SWIMMING/LAP_SWIMMING + pool; run → RUNNING + STREET/TRACK/TRAIL (new `Workout.setting` or default STREET); bike → CYCLING + ROAD/INDOOR_CYCLING. Target map: Swim→SWIM_STROKE, Pace→SPEED, Power→POWER, HR→HEART_RATE. Round-trip tests per sport (mirror the goblin test).
- **Phase 3 — Parser + schemas**: unit suffixes (`km`, `mi`, `k`); `schema::{run(), bike()}`; `documents/run-fixtures/`, `documents/bike-fixtures/`; `infer` learns pace/power markers (likely no new tables — verify before adding fields).
- **Phase 4 — Scraper + CLI**: new `Site` impls per sport (sources TBD — Hal Higdon-style / TrainerRoad-style); `2fit-gen --sport run|bike|swim` (pool swim-only, base generalized); `parse_with_schema` unchanged.

Order: run first (SPEED = smallest encoder delta), then bike (POWER/CADENCE). 2–4 atomic green commits per phase.
