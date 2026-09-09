# Checkpoint (2026-09-09) — Schemas + inference done; all spec items closed

## Done this turn — commit `7deb2a0`

- **`fit_core::FormatSchema`** (core): 14-field vocabulary-as-data (section labels, total/repeat/annotation markers, strokes, freestyle/drill words, interval/clock/base markers, rest/recovery words, count separator; serde; case-insensitive whole-token match + helpers). Dropped `#[non_exhaustive]` — it blocks the struct expression the swimdojo constructor needs; extensibility is via new tables, not subclassing.
- **Parser over schema**: `swimdojo::parse` = `parse_with_schema(…, schema::swimdojo())`; all literals (`warm up`, `through`, `—>`…, strokes, `kb`/`b`, `rest`, `easy`, ` x `, `@`) now flow from the schema. Only `Set N` stays structural (numeric). All 19 generator tests pass unchanged — behavior identical by construction.
- **`fit_generator::parser::schema::swimdojo()`**: swimdojo vocabulary as pure data + JSON round-trip test + schema-drives-parser test.
- **`fit_scraper::schema::infer(name, samples)`** (+ `Inference{gaps,unclassified}` display): per-line-class voting; repeat detector requires bare `N x WORD` (evidence: `500 swim`/`10 x 200` false positives found by test); annotation quotes feed stroke tables (`800 IM (200 of each stroke)` — needed since fixtures' only `back`/`stroke` live in annotations); rest words scan both sides of `@`; drill lines with intervals record both. Verified: zero gaps on 3 fixtures, inferred schema parses all 3 **equal** to default, `2fit-gen --format` byte-identical (1719 B). `2fit-scrape schema FILE…` → stdout JSON, report to stderr; `2fit-gen --format schema.json`.
- `documents/swimdojo-site.md`: schema/inference section appended.
- Workspace: 49 tests (18 core + 21 generator + 10 scraper), clippy + fmt clean. New dep edges: generator→serde_json, scraper→fit_generator (schema ownership lives with the parser).

## Next: user review — every spec item is implemented

- Generator 1–7: parser + encoder, IDL (`documents/idl.md`), swimdojo format first, bin+lib, file/stdin/stdout, terms defined, schemas stored. Scraper 1–3: layout explored (`documents/swimdojo-site.md`), extensible (`Site` trait), inference done.
- Suggested follow-ups (not started): more sites' schemas, `2fit-gen` reading schema by site name, TRY/FIXME audit of `unwrap_or` fallbacks in schema paths.

(Tasks: parser[x] encoder[x] cli/lib[x] round-trip[x] scraper[x] infer[x])
