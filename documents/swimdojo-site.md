# swimdojo.com — site notes for the scraper

Collected 2026-08-30 (turns 1–2 exploration). Platform: Squarespace.

## URL structure

- Listing: `https://www.swimdojo.com/workouts`
- Detail: `https://www.swimdojo.com/workouts/YYYY/M/D/slug`
  (e.g. `/workouts/2023/5/15/how-to-beginners` — date path + slug).
- Filters (query params on listing):
  - "By Distance", "By Level", "By Stroke" — in-page filter UI.
  - `?tag=<tag>` — filter by tag.
  - `?author=<id>` — filter by author (numeric id).
  - `?offset=<epoch-ms>` — pagination cursor (millisecond timestamp of the
    last item), not page numbers.

## Body HTML location

Workout text lives in:

```
div[data-layout-label="Post Body"]
  → .sqs-block.html-block
     → div.sqs-html-content        // inner HTML of the workout notation
```

## Notation

Grammar extracted to `documents/swimdojo-grammar.txt` (from the "How To,
Beginners" article). Parsers target the notation, not the HTML.

## Scraping constraints to verify during impl (not yet confirmed)

- User-Agent / robots.txt: Squarespace may block non-browser UAs — test with
  a real browser UA; add `--user-agent` flag.
- Rate limiting: none observed; be conservative (1 req/s).
- Detail pages may include extra blocks (images, links) — only the
  `html-block` content div holds notation.
- Filters may be client-rendered (JS) — in that case fetch listing with query
  params server-side and filter client-side in code.
