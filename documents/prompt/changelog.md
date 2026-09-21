# Changelog

- Turn 18 (2026-09-21): ramp expansion — `beb67e0` (per-minute `PowerRamp` expansion for text + `.zwo` Ramp; `split_count` anchored fix). See checkpoint.md.
- Turn 17 (2026-09-21): named race pace — `3d4ca88` (`Workout.race_paces` + `--race-pace NAME=PACE` + trailing/`@` resolution). See changelog detail below.
- Turn 16 (2026-09-21): review — `a8726cb` removes `Box::leak` note plumbing (owned `String`s) + true last-wins targets (winner source word not notes). See changelog detail below.
- Turn 15 (2026-09-21): user decisions 1–4 done (all green: 83 tests, clippy + fmt clean) —
  `bc0f071` sub-sport (#4: `SubSport` + `Workout.sub_sport` + `effective_sub_sport()` + encoder map + `2fit-gen --sub-sport` presets);
  `a986512` per-workout Higdon (#1: `plan_days()`/`day_body()`/`fetch_days()` + `fetch --per-day [--out-dir] [--skip-rest]` + `2fit-gen --name`);
  `8aecd29` target flags (#3: `Workout::{run_base,bike_ftp}` + `parse_target_text` direct `250W`/`Z4`/`95rpm`, gated `110% FTP`/`85% of 1mi pace` + `2fit-gen --run-base/--ftp`);
  `988ca00` bike source (#2: `.zwo` file import `parser::zwo` + `2fit-gen --zwo`, no new scraper; survey in `run-bike-notation.md` §3).
  Deferred: named race pace, ramp expansion, `FreeRide` open steps, `<textevent>` cues.
- Turn 14 (2026-09-21): Phase 4 scraper + CLI — `2843160` Higdon site, `82f550c` format presets. See checkpoint.md for detail.
- Turn 13 (2026-09-21): Phase 3 parser + schemas — `5f2dcb7` units/durations, `287c047` fixtures, `f5f36f8` higdon_run/zwift, `6ef165f` infer. See checkpoint.md for detail.
- Turn 12 (2026-09-21): Phase 2 CLI + per-sport round-trips (`9c86e03`). See checkpoint.md for detail.
- Turn 11 (2026-09-20): Phase 1 Core IDL — `d15fc11` units, `bf529c9` sport/pool, `0713617` target/timed. See checkpoint.md for detail.
- Turn 10 (2026-09-20): Multisport Phase 0 survey. See checkpoint.md for detail.

- Turn 9 (2026-09-09): Schemas + inference (`7deb2a0`). See checkpoint.md for detail.
- Turn 8 (2026-09-08): Scraper on the Squarespace RSS feed (`9e9d968`). `Swimdojo::list(ListFilter{tag,author,limit,query})` with `?tag=`/`?author=` server-side (verified exact-match + composable with `?offset=` cursor; 20/page, `published<=cursor` skip, 1 s politeness) and query/limit client-side; `normalize_body` (feed `content:encoded` → notation; box/sea-otter byte-exact vs fixtures, goblin modulo `'`/`'`); `fetch` detail fallback via Post-Body content div (live box-crab fetch ≡ fixture, pipes to `2fit-gen` → 1298 B `.fit`). Deps: `ureq 3` + `rss 2` + `chrono 0.4` + `html-escape 0.2`. 6 scraper tests + 1 CLI test. Fixed live-found non-ASCII panic in the div scanner (advance by `len_utf8`).
- Turn 7 (2026-09-09): CLI + round-trip (`21a0f58`). `2fit-gen`: `-f/--file` (default/`-` = stdin), `-o/--out` (default stdout), `--pool` (default `25yd`), `--base` (optional). Lib `parse_str` + `fit_core` re-export. Fixture round-trip test (goblin → `to_fit` → `Decoder`). Bug: sea-otter's concatenated notes (~305 B) exceeded FIT's 255 B string limit → `fit_str()` truncation at char boundary. 38 tests, clippy/fmt clean.
- Turn 6 (2026-09-08): `.fit` encoder (`f21687c`). `fit::to_fit(&Workout) -> Result<Vec<u8>, EncoderError<io::Error>>`: FileId (WORKOUT / DEVELOPMENT-255 / "2fit") + Workout (SWIMMING, LAP_SWIMMING, pool m×100, METRIC/STATUTE hint) + one WorkoutStep per `flat_steps()`. Stroke targets 0/1/2/3/6, omitted for `None`/`Any`. 34 tests, clippy + fmt clean.
- Turn 5 (2026-09-01): swimdojo parser (+906). Line classifier, step builder, interval parser (`@ kb`, `@ b±offset`, fixed times). Notes attach to previous step. 3 fixtures normalized; 12 parser tests, each reconciling to its stated total (Goblin 1400, Box Crab 1000, Sea Otter 800). Updated `documents/swimdojo-site.md`. Commits: `1f3d097`, `3e08042`, `2cea9c5`.
- Turn 4 (2026-08-31): Rust 1.98 toolchain; rustyfit `0.6.1 → 0.10.2`, edition 2024. Core IDL (`core/src/lib.rs`): 16 unit tests. rustyfit encoding API verified → documents/rustyfit.md.
- Turn 3 (2026-08-30): FIT crate = **rustyfit v0.10.2**. Plan written (workspace layout, core IDL, base-math rule, .fit mapping). No code yet.
- Turn 1–2: Research only. Explored swimdojo.com layout + notation; extracted grammar; surveyed Rust FIT crates; shortlisted rustyfit vs fit-sdk-rust.
