# myswimpro.com (second site — proves the abstraction)

WordPress blog (`blog.myswimpro.com`), scraped via the WP REST API
(`/wp-json/wp/v2`). Surveyed 2026-09-09: 10 `Workout of the Week`
articles (category id 15, 27 posts) + `Whiteboard Wednesday` (id 14,
technique videos, no sets — excluded).

## Source mechanics (vs swimdojo's Squarespace feed)

- **Listing**: `GET /posts?categories=15&per_page=N&_fields=link,title,date,content,categories`
  (`Myswimpro::list`; `search=` server-side for `--query`, title/summary
  refine client-side like swimdojo). No cursor pagination needed (27 posts
  fit one page; `want.min(100)`). Verified live: `--site myswimpro list
  --limit 2` returns the two 2022 trials.
- **`date` is local wall time** (`2022-06-09T12:40:42`, no offset), not
  RFC 3339 — parsed via `NaiveDateTime` as UTC (first attempt with
  `parse_from_rfc3339` returned `None`, caught by `posts_parse`).
- **Fetch**: `GET /posts?slug=<slug>` → `content.rendered` (no theme
  chrome). Raw-HTML fallback exists but is untested live — the WP path
  always wins. Verified: `fetch <1800-url> | 2fit-gen → 961-byte .fit`.
- **robots**: WP API is an open API surface (no disallow observed);
  same 1 req/s politeness + browser UA as swimdojo.

## Notation divergences (all evidenced in `documents/myswimpro-fixtures/`)

4 live-fetched fixtures (`1800-variety`, `comeback-1200`, `usrpt-1500`,
`scull-1250`):

- Sections: `Warmup`, `Pre-Set (2x)`, `Main Set (3x)`, `Cool Down`, plus
  drill/set sub-sections (`Kick & Drill`, `IM Set (x2)`, `Scull Set`),
  `Round 1: …` equipment notes (→ `—>` annotations), `The Workout (30
  seconds rest …)` verbatim header (tethered). A bare `The Workout` h4 is
  an article subheading, skipped.
- Steps: `×` (U+00D7) count separator, spaceless counts (`5x60m`),
  unit-suffixed distances (`60m`, `50s` — pool supplies unit, `s` =
  drill shorthand), equipment words (`Kick`, `Pull`, `Drill`, `with
  Fins/Paddles/Snorkel` — notes, no IDL stroke), dryland cross-training
  (`+ 5 Pushups/Squats` — dropped, no swim distance), time-based tethered
  steps (`1 x 3:00 Free`), prose intensities (`Easy`, `Descend 1-4`,
  `Best Average` — notes).
- Out of scope (dropped, documented): narrative open-water posts (0
  digit-led steps), dryland-only challenges, `100 strokes easy`
  stroke-count steps (no distance).

## Parser changes the site forced (all covered by tests)

- `split_count`: schema separator first, then ` x `/`×`/spaceless-`NxM`
  (`5x60m` → count 5, distance `60m`); `3x through:` still not a count
  (no digit after `x`). Swimdojo's 19 tests unchanged.
- Distance tokens strip one trailing `m`/`M`/`s` (`50s` → 50).
- Rest marker is the literal word `rest`, not `rest_words[0]` — real bug
  found by parity check: inference's BTreeSet order put `freestyle`
  first, turning `300 Freestyle @ 5:00 …` into a 300-second rest.
- `schema::myswimpro()`: pre-set/post-main labels, extra drill nouns,
  `×` separator. `infer` learned: repeat-suffixed labels (`(3x)`/`(x2)`),
  `×`-split tokenization, `freestyle`/`drill(s)` vocabulary,
  digit-token filtering in rest words.

## Inference on the fixtures

`2fit-scrape schema documents/myswimpro-fixtures/*.txt --name myswimpro`:
labels incl. `kick & drill`/`im set`/`scull set` (→ `Main`), strokes
back/breast/fly/IM, `freestyle`/`drill`/`scull`, `easy`; gaps honestly
reported (`total_prefix`, `repeat_word`, `clock_prefix`,
`base_marker` — MSP uses none). Inferred schema parses all 4 fixtures
to identical distances/strokes/intervals as default; only note text
differs (`Freestyle` consumed as stroke-adjacent vocabulary) — so
`.fit` bytes differ at notes level, not structurally.
