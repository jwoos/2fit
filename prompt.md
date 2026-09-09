# Prelude
This file is your file that you are free to edit. DO NOT EDIT above the "OKAY TO EDIT BELOW THIS" line. You are free to do anything to the file below the line. Use this to keep track of progress, knowledge, and manage work. If you think there is an edit i should make ABOVE the line, put it in a file called prompt-edits.md. At each turn, I will review and update this section as necessary.

# Intro
I want to work on a program which has two major components:

1. A program which takes workouts written in a certain format, parses it, and generates a .fit file as an end product. Referred to as generator below.
2. a program which can scrape the workouts from https://www.swimdojo.com/workouts. referred to as scraper below.

# Guidelines
1. Create atomic commits that compile and function.
2. Break down the work into tasks and keep track of them in this file
3. it's okay to challenge what i tell you - i may be wrong. 
4. always give me data backed decisions. point to evidence.
5. keep track of the turn below.
6. Feel free to expand this document via links to other files, stored in documents/. Store all necessary references and resources there.
7. Write down all decisions and knowledge so that you don't have to rederive the knowledge in the future.

# Specifications
## Overall
1. Write this in Rust
2. Make each component modular and stick to DRY principles.
3. Abstract concepts as necessary to allow for extensibility and such.

## Generator
1. Split it into two components: parser and .fit generator.
  1. the parser is responsible for parsing the input into a standard format that the program uses.
  2. the generator will take the standardized input and generate the .fit file.
2. Define a standardized format after researching what workouts look like written down.
3. Must be able to parse various formats - most of which will be defined later. Start with the swimdojo format.
4. This should be able to be used both as a standalone binary and as a library.
5. Must be able to handle various inputs, including file and stdin (if binary, otherwise just string)
6. Define terms used in workouts and generate a definition to follow - might make sense to generate an IDL.
7. Store workout formats as schemas.

## Scraper
1. Explore swimdojo website and get an idea of its layout as well as the format of the workouts. We should allow filtering the page and such
2. Make this extensible so that the scraper can scrape other sites in the future. 
3. It should be able to infer a workout format from a site, generating a schema for the generator to consume. 

OKAY TO EDIT BELOW THIS

TURN: 9

# Checkpoint (2026-09-09) — Schemas + inference done; all spec items closed

## Done this turn — uncommitted (working tree has these changes)
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

## Done this turn — commit `9e9d968` (turn 8: scraper RSS)

## Done this turn — uncommitted (working tree has these changes)
- **Scraper** `scraper/` on the Squarespace RSS feed (`/workouts?format=rss`): `Swimdojo::list(ListFilter{tag,author,limit,query})` with `?tag=`/`?author=` server-side (verified exact-match + composable with `?offset=` cursor; 20/page, `published<=cursor` skip, 1 s politeness) and query/limit client-side; `normalize_body` (feed `content:encoded` → notation; box/sea-otter byte-exact vs fixtures, goblin modulo `'`/`'`) incl. `<li>`/`<h3>` for the open-water outlier (Hourglass Dolphin); `fetch` detail fallback via Post-Body content div (live box-crab fetch ≡ fixture, pipes to `2fit-gen` → 1298 B `.fit`). Deps: `ureq 3` + `rss 2` + `chrono 0.4` + `html-escape 0.2`. 6 scraper tests (5 lib incl. offline `tests/data/` snapshots: feed.xml, detail page, 4 body HTMLs + 1 CLI test). Fixed live-found non-ASCII panic in the div scanner (advance by `len_utf8`).
- `documents/swimdojo-site.md`: appended verified implementation notes (feed params, pagination, body rules, robots: `?tag=`/`?author=` disallowed, `format=rss` not).
- Workspace: 44 tests, clippy + fmt clean. Live-verified: `list --limit 3`, `list --tag Triathlon --limit 3`, `fetch 2021/2/16/box-crab`.

## Pick up next: schema inference (the remaining scraper spec item)
- Spec: "infer a workout format from a site, generating a schema for the generator to consume" (Generator spec 7: "Store workout formats as schemas"). Scaffolding exists: `scraper/src/schema/mod.rs` is still a stub doc-comment; `site` trait (`Site`/`ListingItem`/`ScrapedWorkout`/`ListFilter`) is the extensibility seam for future sites.
- Open design question (no code yet): what a "schema" is — JSON grammar for the parser? IDL validation rules? Decide with data from the notation variants in `documents/swimdojo-grammar.txt` before building.

(Tasks: parser[x] encoder[x] cli/lib[x] round-trip[x] scraper[x] infer[ ])

## Done this turn — commit `21a0f58` (turn 7: CLI + round-trip)

## Done this turn — uncommitted (working tree has these changes)
- **`2fit-gen` CLI** `generator/src/main.rs` (~140 lines): clap-derive args `-f/--file` (default/`-` = stdin), `-o/--out` (default stdout), `--pool <Nyd|Nm>` (default `25yd`), `--base <m:ss|:ss|secs>` (optional). Errors via `anyhow` → stderr + non-zero exit. 2 unit tests (`pools`, `bases`). Smoke-tested: all 3 fixtures encode (goblin 1719 B, box-crab 1298 B, sea-otter 797 B); stdin ≡ file output.
- **Lib sugar** `generator/src/lib.rs`: `pub fn parse_str(text, pool, base100)` over `parser::swimdojo::parse` + `pub use fit_core`.
- **Round-trip test** `generator/tests/round_trip.rs`: parse goblin-shark → `to_fit` → `Decoder`; asserts step count, pool_length 2286 + STATUTE, `500 swim` → 45720 + no target, `800 IM @ b+120` → IM target 6.
- **Bug found by CLI smoke test (data-backed)**: sea-otter's 9×50 repeat concatenates 4 `—>` notes (~305 B) into one FIT `notes` field → rustyfit encode error "value's size in bytes exceeds 255 bytes limit" (FIT `string` max = 255 B incl. NUL). Fix: `fit_str()` truncates all FIT strings to 254 B at a char boundary (name/description/notes) + unit test incl. multi-byte `é` case. Fixture evidence: sea-otter lines 8–11.
- Test-name correction: goblin's second step displays `800 yd IM @ b+120` (offset `2:00`→`+120`, per `IntervalSpec::Display`), not `@ b`.

Workspace: 38 tests (18 core + 17 generator-lib + 2 CLI + 1 round-trip), clippy + fmt clean.

## Pick up next: Scraper (listing + detail + filters)
- Per `documents/swimdojo-site.md`: listing `https://www.swimdojo.com/workouts` (+ `?tag=`, `?author=<id>`, `?offset=<epoch-ms>` filters, By Distance/Level/Stroke facets), detail `/workouts/YYYY/M/D/slug`, body HTML `div[data-layout-label="Post Body"]` → `.sqs-block.html-block` → `div.sqs-html-content`. Emit normalized notation text (the `documents/swimdojo-fixtures/*.txt` form). Check what HTTP/HTML crates are already vendored before adding deps.
- Then site-format→schema inference (later).

(Tasks: parser[x] encoder[x] cli/lib[x] round-trip[x] scraper[ ] infer[ ])

## Done this turn — commit `f21687c` (working tree clean)
- **`.fit` encoder** `generator/src/fit/mod.rs`: `to_fit(&Workout) -> Result<Vec<u8>, Error>` (Error = `rustyfit::EncoderError<std::io::Error>`). 4 new tests; workspace 34 (18 core + 16 generator); clippy + fmt clean.
- `documents/rustyfit.md`: all three open items RESOLVED (SwimStroke table, capabilities, writer).

Encoder decisions (data-backed; source = rustyfit registry source, tests round-trip via rustyfit's own `Decoder`):
- Writer: `FromStd::new(Cursor::new(&mut buf))` (dep `embedded-io-adapters 0.7` feature `std`, added to workspace + generator) — `enc.encode(writer /*by value*/, &mut fit)`, bytes in `buf`.
- `FileId`: type WORKOUT, `Manufacturer::DEVELOPMENT` (255, the reserved development vendor — the non-OEM value), product_name "2fit".
- `pool_length` always meters×100 (profile unit); `pool_length_unit` is a *display hint*: METRIC for m pools, STATUTE for yd pools (25 yd → 2286, rendered 25 yd not 22.86 m).
- Stroke target (SWIM_STROKE=11): Free/Back/Breast/Fly/IM → 0/1/2/3/6 (crate `typedef::SwimStroke`, verified; IM defensive since 100/200/300 IMs are broken down by `flat_steps`); `None` (swimdojo freestyle-default), `Any` (no single code), future strokes → target omitted (invalid `u8::MAX` skips on encode).
- `capabilities` = 0 (UINT32Z invalid=0 → skipped); decoder tolerance proven by the round-trip test.
- Interval paces are not expressible on FIT `WorkoutStep` (only a DISTANCE/TIME duration + one target) → they survive only in `wkt_step_name` (step name = core `Step::display()`, e.g. `100 yd @ 2:00`).
- Core `Unit`/`Stroke`/`Intensity` are `#[non_exhaustive]` → wildcard arms with conservative fallbacks (METRIC / omit target / OTHER).
- Conversion: meters `value.saturating_mul(100)`; yards `(v*9144+50)/100` u64-rounded.

## Pick up next: CLI + lib exports, then round-trip fixture test
- `generator/src/main.rs` (still "not implemented yet" stub): clap-derive `2fit-gen`: `-f/--file <path>` (default stdin; `-` = stdin), `-o/--out <path>` (default stdout), `--pool <Nyd|Nm>` (default `25yd` — the notation's assumed pool; parser takes it), `--base <m:ss>` (optional; required iff text has `@ b`). Flow: read → `parser::swimdojo::parse(&text, pool, base100)` → `fit::to_fit(&workout)` → write bytes; errors via `anyhow` to stderr + non-zero exit.
- `generator/src/lib.rs`: consider a `pub fn parse_str(...)` sugar + re-export of `fit_core` items for library users (modules `fit`/`parser` already public).
- **Round-trip task**: `parse`(a real fixture, e.g. `documents/swimdojo-fixtures/goblin-shark.txt`) → `to_fit` → rustyfit `Decoder`; assert num_valid_steps + spot-check step durations/targets + pool_length. Closes "Generate a sample .fit and verify" (fixtures' pool units: goblin/box-crab 25 yd, sea-otter 50 m — check fixture headers before asserting).
- Then Scraper (listing+detail+filters) → later the site-format→schema inference.

(Tasks: parser[x] encoder[x] cli/lib[ ] round-trip[ ] scraper[ ] infer[ ])

# Plan
Goal: two Rust binaries in one Cargo workspace sharing a core "IDL" + a rustyfit-based .fit encoder. Decision: use **rustyfit v0.10.2** (see Decisions).

## Architecture (Cargo workspace)
- `core/` (lib `fit_core`): parse-independent domain types = the IDL (Workout/Step/Stroke/Interval/Pool, units, time).
- `generator/` (lib + bin `2fit-gen`):
  - `parser/`: format-specific parsers → core. First: `swimdojo`.
  - `fit/`: core → .fit bytes via rustyfit.
  - CLI: `--file <path>` or stdin (`-`/no arg) → `.fit` (file or stdout). Also usable as a library (`parse_str`, `to_fit`).
- `scraper/` (lib + bin `2fit-scrape`): swimdojo scrape + filters (distance/level/stroke/tag/author/pagination); emit normalized notation text; later: infer a site's format → schema (JSON) for the parser.

## Decisions (data-backed)
- FIT crate = **rustyfit v0.10.2**. Evidence: it *encodes* (Decoder+Encoder, FIT Protocol v2, `Encoder`/`EncoderBuilder`/`StreamEncoder`), and its `mesgdef` structs carry the swim-relevant fields we need — `Workout:{pool_length, pool_length_unit, wkt_name, wkt_description, sport}`, `WorkoutStep:{duration_type, target_type, intensity, wkt_step_name, notes}`, `WktStepDuration::DISTANCE`, `WktStepTarget::SWIM_STROKE`. `fit-sdk-rust` also encodes but is younger (0.2.x, crate named `fit`).
- Standard format is the IDL: a Workout = metadata + an ordered list of steps; a step is single-distance / repeat / breakdown / rest / recovery. FIT has **no repeat primitive** — repeats are expanded to concrete steps (`num_valid_steps` counts them).
- Rest/intervals (swimdojo `@`) are first-class in the IDL, then emitted as FIT `TIME` steps.

## Standard format (IDL)
Canonical spec + evidence: **documents/idl.md** (implemented in `core/src/lib.rs`, 16 unit tests).
Deviations from the old draft, with reasons, are noted there (notably: `Interval` split into `IntervalSpec::Fixed | Base{offset}` + `Workout.base100`, since bases are swimmer data, never written in the workout).

## .fit mapping (core → rustyfit)
Verified against rustyfit 0.10.2 source: **documents/rustyfit.md** (constants: sport=swim 5, sub_sport=lap_swimming 17, WktStepDuration TIME 0/DISTANCE 1, WktStepTarget SWIM_STROKE 11, pool_length scale×100, Intensity 0..6). Open items (swim-stroke target codes, `capabilities` bits, in-memory `Write+Seek` writer) live there too.

# Tasks
- [x] Research swimdojo layout + notation grammar; extract grammar → documents/swimdojo-grammar.txt (was /tmp/sd-howto.txt, now persisted)
- [x] Evaluate FIT crates (rustyfit vs fit-sdk-rust) → choose rustyfit v0.10.2
- [x] Scaffold Cargo workspace (`core`, `generator`, `scraper`) + README
- [x] Define core IDL types (Workout/Section/Step/Stroke/Interval/Pool, units) + unit tests (16 passing)
- [x] Generator: swimdojo parser → core (distance, `@` intervals incl. base math, repeats, IM, recovery) — `parser::swimdojo`, 12 tests incl. 3 real fixtures
- [x] Generator: rustyfit encoder core → .fit (Workout + WorkoutStep), file/stdout — `fit::to_fit` (CLI file/stdout part is the next CLI task)
- [ ] Generator: CLI (file + stdin) + library exports; parse tests on real swimdojo texts
- [ ] Generate a sample .fit and verify by decoding (round-trip), spot-check fields
- [ ] Scraper: swimdojo scrape (listing + detail) + filters (distance/level/stroke/tag/author/pagination)
- [ ] Scraper: site-format inference → schema (JSON) for the parser (later)

# Bugs/Issues
(none yet)

# References
- documents/idl.md — IDL spec + decisions (canonical; links from Plan above)
- documents/rustyfit.md — rustyfit 0.10.2 encoder API, verified from crate source
- documents/swimdojo-grammar.txt — swimdojo notation grammar (persisted; was `/tmp/sd-howto.txt`)
- documents/swimdojo-site.md — swimdojo site layout/filters for the scraper (detail URL + body-HTML block confirmed against 3 real pages, turn 5)
- documents/swimdojo-fixtures/{goblin-shark,box-crab,sea-otter}.txt — real workouts normalized to notation; parser regression fixtures (assert stated totals 1400/1000/800)
- swimdojo workouts: https://www.swimdojo.com/workouts (detail: `/workouts/YYYY/M/D/slug`; filters By Distance/Level/Stroke, `?tag=`, `?author=<id>`, `?offset=<epoch-ms>`). Body HTML: `div[data-layout-label="Post Body"]` → `.sqs-block.html-block` → div.sqs-html-content
- rustyfit (chosen): https://docs.rs/rustyfit — v0.10.2, encode+decode; WorkoutStep: https://docs.rs/rustyfit/latest/rustyfit/mesgdef/struct.WorkoutStep.html
- fit-sdk-rust (rejected): https://docs.rs/fit-sdk-rust — std crate named `fit`, encodes, younger.

# Changelog
- Turn 6 (2026-09-08): Implemented the `.fit` encoder (`generator/src/fit/mod.rs`, commit `f21687c`). `fit::to_fit(&Workout) -> Result<Vec<u8>, EncoderError<io::Error>>` builds `FileId` (WORKOUT / DEVELOPMENT-255 / "2fit") + `Workout` (SWIMMING, LAP_SWIMMING, name, description, pool_length m×100, METRIC/STATUTE display hint) + one `WorkoutStep` per `flat_steps()` entry: DISTANCE (m×100, yards rounded via (v·9144+50)/100) or TIME (secs), SWIM_STROKE target 0/1/2/3/6 (omitted for `None`/`Any`/future strokes), intensity 1:1, `wkt_step_name` = core `Step::display()` (so interval paces survive in the name), notes carried. Core enums are `#[non_exhaustive]` → conservative wildcard fallbacks (METRIC / omit / OTHER). Writer = `FromStd::new(Cursor::new(&mut Vec<u8>))` (new dep `embedded-io-adapters 0.7`, feature `std`, added to workspace + generator — same dep rustyfit's own tests use). 4 tests encode then round-trip through rustyfit's `Decoder`, asserting the `.FIT` magic and every mapped field incl. STATUTE display + pool_length 2286/5000. Clippy caught a real bug: `value * 100` in u32 panics on overflow in debug builds → `saturating_mul`. Fixed stale registry facts in `documents/rustyfit.md` and closed all 3 open items (actual `SwimStroke` table; capabilities=0 skipped as UINT32Z-invalid; writer resolved, no custom adapter needed). Workspace: 34 tests, clippy + fmt clean.
- Turn 5 (2026-09-01): Implemented the swimdojo parser (`generator/src/parser/swimdojo.rs`, +906). Line classifier (blank → `TOTAL:` → section label → bare/parenthesized subtotal → `N x through:` → `—>` annotation → digit-led step → prose), step builder (counted set, `N x through:` block, drills `bobs`/`sculls` → `Step::Technique`, rest/easy, distance), interval parser (`@ kb`, `@ b`, `@ b +:30` / `+2:00`, fixed `2:00`/`:30`/`45`). Notes attach to the previous step (RepeatStep/RecoveryStep gained `notes` slots). 3 real pages normalized to `documents/swimdojo-fixtures/*.txt`; 12 parser tests pass, each reconciling to its stated total (Goblin 1400, Box Crab 1000, Sea Otter 800). Caught a test-data typo via hexdump: Goblin line 16 was written `@ b +:+30` but the site says `@ b +:30` — fixed the fixture, not the code. Updated `documents/swimdojo-site.md` (confirmed detail URLs + body block classes, subtotal-vs-step-sum caveat, notation-variant list). Workspace green: 30 tests (18 core + 12 generator), `cargo clippy` + `cargo fmt` clean. Commits: `1f3d097` (core Technique), `3e08042` (core notes), `2cea9c5` (parser + fixtures).
- Turn 4 (2026-08-31): User added Guidelines 6–7 (documents/, record knowledge) and updated the Rust toolchain (rustc/cargo now 1.98; rustyfit 0.10.2 requires MSRV 1.93 + edition 2024 — that's why the bump was needed). Bumped workspace rustyfit `0.6.1 → 0.10.2` (latest on crates.io; the lockfile already had 0.10.2), set edition 2024 + `rust-version 1.93`; scaffold builds clean. Implemented core IDL (`core/src/lib.rs`): Unit/Distance/Seconds/Pool/Stroke/Intensity/IntervalSpec/Part/DistanceStep/RepeatStep/BreakdownStep/RecoveryStep/Step/SectionLabel/Section/Workout/FlatStep/Error — 16 unit tests pass (incl. base-math "do not scale the offset", IM breakdown, repeat expansion, serde round-trip). Verified rustyfit encoding API from the registry source → documents/rustyfit.md (key: `Encoder::encode(W: Write+Seek, &mut FIT)`; embedded-io has `Write` for `Vec<u8>` but no `Seek` ⇒ plan a tiny in-memory `Write+Seek` writer; `SubSport::LAP_SWIMMING=17` is the pool swim, not a `POOL` const; `pool_length` scaled ×100).
- Turn 3 (2026-08-30): User added `Changelog` section, `TURN:` field, and Guideline 5 (track turn). Decided FIT crate = **rustyfit v0.10.2** (evidence: encodes + `pool_length` + `DISTANCE`/`SWIM_STROKE` fields). Wrote Plan (workspace layout, core IDL, base-math rule, .fit mapping) and Tasks above. No code yet.
- Turn 1–2: Research only. Explored swimdojo.com (Squarespace) layout + notation; extracted grammar to `/tmp/sd-howto.txt`; surveyed Rust FIT crates; shortlisted rustyfit vs fit-sdk-rust.

