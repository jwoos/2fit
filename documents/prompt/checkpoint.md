# Checkpoint (2026-09-21) — Turn 13: Phase 3 parser + schemas done; NEXT: Phase 4 scraper + CLI polish

## Done — 4 atomic commits (all green: 68 tests, clippy + fmt clean)

- **`5f2dcb7` explicit units + durations**: `suffixed_distance()` (unit beats
  pool: `3 mi`, `4.8 km`→4800 m, `5km`/`5k`/`400m`, `400 5K pace`→400 m),
  `Distance::{from_km_decimal, from_miles_decimal}` (integer-only;
  `3.5 mi`→5633 m), `leading_duration()` (`35 min tempo`, `30:00 easy` →
  `Step::Timed`, target `None`), `@`-after-run-distance kept as note
  (`3 mi run @ 8:00` → notes, `interval: None`). 3 parser tests.
- **`287c047` fixtures parse**: `documents/run-fixtures/` (3 Higdon weeks +
  Zwift best-mile) + `documents/bike-fixtures/` (Zwift ramp excerpt);
  spaceless `5min`/`1min` durations; all 5 fixtures `2fit-gen --sport run` OK.
- **`f5f36f8` schemas**: `schema::higdon_run()` + `schema::zwift()` (vocabulary
  only — no new `FormatSchema` fields); fixture tests per schema; real bug
  fixed: `@ 85rpm` after a duration misread as swim base (`85` + `rpm` tail)
  → `@`-after-duration now routes straight to notes.
- **`6ef165f` infer**: run/bike filler (`run`, `tempo`, `fast`, `ride`,
  `walk`, `jog`, `cross`, `cooldown`) votes `freestyle_words`; swim-only
  tables honestly gap on run samples; `infers_run_filler_as_freestyle` test.
- Interval-style evidence now fetched (Higdon 10K Intermediate grid + Zwift
  FTP/101-Running via whatsonzwift; `run-bike-notation.md` §2 rewritten):
  `%FTP`/`%pace`/named pace/watts stay **notes** (need athlete thresholds;
  `run_base`/`bike_ftp` still deferred).

## NEXT: Phase 4 — Scraper + CLI (planned, no code)

- New `Site` impls per sport (Higdon grid? Zwift/whatsonzwift excerpts?
  sources TBD — Phase 3 used hand-normalized cells, no scraper yet);
  `2fit-gen --sport` exists (Phase 2); `--base` generalized to pace/FTP
  when a parser emits `Target` (deferred — targets are notes today).
- `parse_with_schema` unchanged (schema-driven parsing held: run/bike are
  new `FormatSchema` values, not a new parser).
