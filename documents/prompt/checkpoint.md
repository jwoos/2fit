# Checkpoint (2026-09-21) — Turn 14: Phase 4 scraper + CLI done; NEXT: user review

## Done — 2 atomic commits (all green: 79 tests, clippy + fmt clean)

- **`2843160` Higdon scraper**: `site::Higdon` (plain-HTML `table.tablesaw`
  grids: miles table wins, km second; `Week N` + bare cells + `—>Rest days:`
  annotations, no day prefixes — parser has no day model; `normalize_cell`
  `6`→`6 mi`; `is_day_marker` Rest/Cross/race names). `list` seeds from
  Novice 1's 14 program links (no server search; tag/query client-side);
  `fetch` parses any plan URL. Offline fixture
  `scraper/tests/data/higdon-novice1.html` (118951 B saved page) + 4 tests
  incl. whole-plan parse via `higdon_run`. Live verified: `list --limit 3`,
  `fetch novice-1` → pipe → `2fit-gen --sport run` → 2546 B `.fit`.
  Wired as `2fit-scrape --site higdon`.
- **`82f550c` `--format` presets**: JSON path or preset name (`swimdojo`,
  `myswimpro`, `higdon-run|higdon|run`, `zwift|bike`); default follows
  `--sport` (swim→swimdojo, run→higdon-run, bike→zwift).
  `resolve_schema()` + `schema_resolution` test; live smoke all 3 defaults.

## State of multisport (Phases 0–4 complete)

- Phase 0 survey → `run-bike-notation.md` (Higdon + Zwift evidence).
- Phase 1 IDL → `Sport`, optional pool, km/mi, `Target`/`Pace`,
  `Step::Timed`, `FlatStep.target` (+ `idl.md` FIT mapping).
- Phase 2 CLI sport + per-sport round-trips.
- Phase 3 parser units/durations + `higdon_run`/`zwift` schemas + fixtures
  + infer filler; `%FTP`/`%pace`/named pace stay notes (thresholds deferred).
- Phase 4 Higdon scraper + format presets (this turn).

## NEXT (deferred, needs user direction)

- More run/bike sources (Zwift/whatsonzwift scraper? TrainingPeaks?
  manana — Phase 3/4 used hand excerpts, no bike scraper yet).
- `run_base`/`bike_ftp` + `Target` resolution (today targets are notes).
- INDOOR_* / TRAIL / TRACK sub-sports (today STREET/ROAD).
