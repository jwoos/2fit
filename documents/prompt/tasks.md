# Tasks

- [x] User decision 1: per-workout Higdon (`fetch_days`, `fetch --per-day/--out-dir/--skip-rest`, `2fit-gen --name`)
- [x] User decision 2: bike source survey → `.zwo` import (`parser::zwo`, `2fit-gen --zwo`, fixtures), no new scraper
- [x] User decision 3: target flags (`Workout::{run_base,bike_ftp}`, `parse_target_text`, `2fit-gen --run-base/--ftp`; HR/pace/power/cadence direct, `%` gated)
- [x] User decision 4: `--sub-sport` (core `SubSport`, encoder map, CLI presets; street/road defaults)
- [x] Multisport Phase 0: survey run/bike notation → documents/run-bike-notation.md; verify FIT run/bike facts in rustyfit source
- [x] Multisport Phase 1: core IDL (`Sport`, optional pool, km/mi units, `Target`/`Pace`, `Step::Timed`, `FlatStep.target`)
- [x] Multisport Phase 2: `2fit-gen --sport` + per-sport round-trip tests
- [x] Multisport Phase 3: parser run/bike units + durations; `higdon_run`/`zwift` schemas + fixtures; infer filler
- [x] Multisport Phase 4: Higdon grid scraper + `--format` presets/defaults

- [x] Research swimdojo layout + notation grammar; extract grammar → documents/swimdojo-grammar.txt (was /tmp/sd-howto.txt, now persisted)
- [x] Evaluate FIT crates (rustyfit vs fit-sdk-rust) → choose rustyfit v0.10.2
- [x] Scaffold Cargo workspace (`core`, `generator`, `scraper`) + README
- [x] Define core IDL types (Workout/Section/Step/Stroke/Interval/Pool, units) + unit tests (16 passing)
- [x] Generator: swimdojo parser → core (distance, `@` intervals incl. base math, repeats, IM, recovery) — `parser::swimdojo`, 12 tests incl. 3 real fixtures
- [x] Generator: rustyfit encoder core → .fit (Workout + WorkoutStep), file/stdout — `fit::to_fit` (CLI file/stdout part is the next CLI task)
- [x] Generator: CLI (file + stdin) + library exports; parse tests on real swimdojo texts
- [x] Generate a sample .fit and verify by decoding (round-trip), spot-check fields
- [x] Scraper: swimdojo scrape (listing + detail) + filters (distance/level/stroke/tag/author/pagination)
- [x] Scraper: site-format inference → schema (JSON) for the parser

# Bugs/Issues

(none yet)
