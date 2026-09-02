# swimdojo.com — site notes for the scraper

Collected 2026-08-30 (turns 1–2 exploration); confirmed against 3 real detail
pages (turn 5). Platform: Squarespace.

## URL structure

- Listing: `https://www.swimdojo.com/workouts`
- Detail: `https://www.swimdojo.com/workouts/YYYY/M/D/slug`
  (date path + slug). Confirmed examples:
  - Goblin Shark — `/workouts/2025/5/19/goblin-shark`
  - Box Crab — `/workouts/2021/2/16/box-crab`
  - Sea Otter — `/workouts/2021/3/26/sea-otter`
- Filters (query params on listing):
  - "By Distance", "By Level", "By Stroke" — in-page filter UI.
  - `?tag=<tag>` — filter by tag.
  - `?author=<id>` — filter by author (numeric id).
  - `?offset=<epoch-ms>` — pagination cursor (millisecond timestamp of the
    last item), not page numbers.
- Listing items link to the detail URL above (server-rendered `<a href>`);
  scrape `href` + title, then fetch each detail page.

## Body HTML location (confirmed)

Workout text lives in:

```
div[data-layout-label="Post Body"]
  → .sqs-block.html-block
     → div.sqs-block-content.sqs-html-content   // inner HTML of the notation
```

- The content div carries both `sqs-block-content` and `sqs-html-content`
  classes; either is a reliable selector for the notation HTML.
- Lines are separated by `<br>`; subtotals are wrapped in `<em>` (a whole-line
  number after the section's steps). Strip tags → the normalized notation
  preserved in `documents/swimdojo-fixtures/*.txt`.

## Normalized notation → parser fixtures

The 3 detail pages were normalized (tags/br stripped, arrows kept) and saved
as regression fixtures for the parser (`fit_generator`: `parser::swimdojo`):

- `documents/swimdojo-fixtures/goblin-shark.txt`
- `documents/swimdojo-fixtures/box-crab.txt`
- `documents/swimdojo-fixtures/sea-otter.txt`

The parser test for each asserts the parsed total equals the page's stated
`TOTAL:` (Goblin 1400, Box Crab 1000, Sea Otter 800).

## Subtotal caveat (important)

A section's **stated** subtotal is *not* always the sum of its parsed steps.
The parser keeps the stated subtotal as data (does not recompute). Examples
seen in the fixtures:

- Sea Otter states its warm-down as `100 easy` with `—> see how you feel`, and
  its set-1 summary `9 x 50 @ :20 rest:` carries the breathing notes via a
  `3x through:` block that adds no distance of its own.
- Counted drills (`3 x 10 bobs`) contribute **0 distance**, so a stated total
  counts only swum strokes — the parser models these as `Step::Technique`.

## Notation variants the parser handles

See `documents/swimdojo-grammar.txt` for the full grammar. Variants confirmed
in the fixtures:

- Intervals: `@ kb` (kickboard / "the clock"), `@ b` (at base/100),
  `@ b +:30` / `@ b +2:00` (base ± offset), fixed times `@ 2:00` / `@ :30` /
  `@ 45`.
- `N x DIST @ ...` — a counted set → `Repeat`.
- `N x through:` — opens a repeat block; following lines are its inner steps
  until the next label/subtotal/end. A `through:` block with no steps (Sea
  Otter's) is dropped and its `—>` notes attach to the previous step.
- `—>` / `–>` / `-->` annotations and trailing prose → notes on the previous
  step; an embedded arrow splits a line into step text + note.
- Drills: `bobs`, `sculls` (counted → `Technique`, 0 distance).
- Rest / easy: `30 seconds rest`, `50 easy`, `100 swim strong, ...`.
- Choices (`100 IM/100 free`) and `stroke` (= any non-freestyle stroke) are
  noted as prose / `Stroke::Any` respectively, not as distinct steps.

## Scraping constraints to verify during impl (not yet confirmed)

- User-Agent / robots.txt: Squarespace may block non-browser UAs — test with
  a real browser UA; add a `--user-agent` flag.
- Rate limiting: none observed; be conservative (1 req/s).
- Detail pages may include extra blocks (images, links) — only the
  `html-block` content div holds the notation.
- Filters may be client-rendered (JS) — in that case fetch the listing with
  query params server-side and filter client-side in code.
