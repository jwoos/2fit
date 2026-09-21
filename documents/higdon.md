# halhigdon.com — grid mechanics, divergences, scraper changes, evidence report

Evidence for the Higdon run scraper (`scraper/src/site/higdon.rs`, Phase 4).
Counterpart to `swimdojo-site.md` (swim) and `myswimpro.md` (second swim site).

## Site mechanics

- Each program page is a static grid: two `table.tablesaw` tables (miles
  first, km second — same cells converted, e.g. `3 mi run` / `4.8 km run`,
  `20` / `32.2`), header `Week | Mon..Sun`, one row per week (Novice 1:
  18 weeks + header = 19 rows/table). Saved evidence:
  `scraper/tests/data/higdon-novice1.html` (118951 B, live fetch 2026-09-21).
- Cells are one workout line each: `<N> mi|km run` (incl. decimals `3.5 mi`,
  `5.9 km`), bare long-run distances (`6`, `20` — unit from the table),
  `Rest`, `Cross`, race names (`Half Marathon`, `Marathon`, `5K Race`,
  `5K Test`, `10K Race`), tempo bounds (`35 min tempo run`, `60 min cross`),
  track reps (`8 x 400 5K pace`).
- Program links are static `training-programs/<group>/<slug>/` hrefs (14 on
  Novice 1; marathon groups + cross-links). There is no listing endpoint or
  server search: `list` seeds from Novice 1's links, tag/query filter
  client-side on title/URL.

## Normalization (lossy by design)

- One plan week → `Week N` + bare cells in Mon..Sun order (day prefixes
  dropped — the parser has no day model; a `Tue:` prefix would become note
  text on the step). Miles table wins (imperial source; km are conversions).
- Bare numbers gain `mi` (`6` → `6 mi`); day markers (`Rest`, `Cross`,
  `*Marathon`, `*Race`, `*Test`) become one `—>Rest days: …` annotation per
  week with day names kept (`Mon Rest, Fri Rest, Sun Cross`). The IDL has no
  rest-day step (`run-bike-notation.md` §1).

## Inference report

- `infer` on Higdon cells learns `run`/`tempo`/`cross` into
  `freestyle_words` (the run "no target" marker); swim-only tables
  (strokes, drills, clock/base) honestly gap. `2fit-gen --sport run`
  defaults to the `higdon-run` schema (vocabulary-only preset), so
  scraped bodies parse without `--format`.

## CLI evidence (live, 2026-09-21)

- `2fit-scrape --site higdon list --limit 3` → 14-title list (Advanced 1/2,
  Alternate, …).
- `2fit-scrape --site higdon fetch <novice-1-url>` → `Week 1 / 3 mi run ×3
  / 6 mi / —>Rest days: …` … (18 weeks).
- Pipe `fetch` → `2fit-gen --sport run -o novice1.fit` → 2546 B `.fit`.
