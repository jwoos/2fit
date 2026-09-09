# Checkpoint (2026-09-09) — Second site (myswimpro) done; abstraction proven

## Done this turn — uncommitted (working tree has these changes)

- **`Myswimpro` site** (`scraper/src/site/myswimpro.rs` + `Swimdojo::blocks/clean_block` now document-order + shared): WP REST API listing (`categories=15`, `search=`, `NaiveDateTime` wall-time dates) + slug-lookup fetch (`content.rendered`, raw-HTML fallback) + normalizer (section headings incl. `Kick & Drill`/`IM Set (x2)`/`The Workout (…)`, `×→ x`, dryland dropped, `Round N` → `—>`). 4 live-fetched fixtures in `documents/myswimpro-fixtures/`; 4 offline tests (posts, body, steps, 3 page fixtures).
- **Parser gaps closed** (all test-covered): multi-separator `split_count` (` x `/`×`/spaceless `5x60m`), unit-suffixed distances (`60m`/`50s`), `schema::myswimpro()` constructor + fixture tests, `infer` learns repeat-suffixed labels/`×` tokens/`freestyle`/`drill(s)`. Real bug fixed: rest marker is literal `rest`, not `rest_words[0]` (inference order put `freestyle` first → 300 m swim became a 300 s rest).
- **End-to-end**: `--site myswimpro list/fetch` live-verified; `fetch …usrpt… | 2fit-gen → 1840 B .fit`. Inferred MSP schema: honest gaps (`total_prefix`, `repeat_word`, `clock_prefix`, `base_marker` — MSP uses none); parses to identical structure, notes-level `.fit` diffs only.
- `documents/myswimpro.md` (new): mechanics, divergences, out-of-scope, parser changes, inference report.
- Workspace: 55 tests (18 core + 22 generator + 15 scraper), clippy + fmt clean. No new deps (serde_json/chrono already vendored).

## Next

- Spec items all closed (turn 9) + second site proven (this turn).
- Suggested: real-device `.fit` check; `--format` by site name; push 8 unpushed commits.
