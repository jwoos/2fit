# Tasks

- [x] Multisport Phase 0: survey run/bike notation → documents/run-bike-notation.md; verify FIT run/bike facts in rustyfit source

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
