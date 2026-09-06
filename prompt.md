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

TURN: 5

# Checkpoint (2026-09-01) — stop here; `.fit` encoder is mid-research

## Done (committed; working tree clean)
- **Core** final: `Step::Technique` (`1f3d097`) + `notes` on `RepeatStep`/`RecoveryStep` (`3e08042`). 18 core tests pass.
- **swimdojo parser** (`2cea9c5`): `generator/src/parser/swimdojo.rs`, 12 tests (grammar + 3 real fixtures in `documents/swimdojo-fixtures/`, each reconciling to stated totals 1400/1000/800).
- **Docs** (`afde2c3`): `swimdojo-site.md` (confirmed detail URLs + Post Body block classes, subtotal caveat, notation variants); `prompt.md` TURN→5, parser task checked, fixtures referenced, changelog.
- Workspace green: 30 tests (18 core + 12 generator); `cargo clippy` + `cargo fmt` clean.

## Pick up next: `.fit` encoder — `generator/src/fit/mod.rs` (still a 1-line stub)
All facts below derived from rustyfit 0.10.2 source *this session* — do not re-derive.

**Deps to ADD** (embedded-io is only transitive via rustyfit now; not in our tree):
- workspace `[workspace.dependencies]`: `embedded-io = "0.7.1"` and `embedded-io-adapters = { version = "0.7.0", features = ["std"] }` (adapters = `FromStd`).
- generator `[dependencies]`: `embedded-io.workspace = true`, `embedded-io-adapters.workspace = true`.
- In-memory writer pattern (from rustyfit's own tests): `let mut w = FromStd::new(std::io::Cursor::new(Vec::new()));` → `Encoder::new().encode(w, &mut fit)?` → recover bytes from `w` (confirm the adapter's accessor, e.g. `into_inner()`/`get_ref()`, when wiring).

**API**: `Encoder::encode<W: embedded_io::Write + Seek>(&mut self, writer: W /*by value*/, fit: &mut FIT) -> Result<(), Error<W::Error>>`. `FIT{file_header, messages: Vec<Message>, crc}` (Default); `Message::from(mesgdef)`.
**Public wrapper**: `fit::to_fit(&Workout) -> Result<Vec<u8>, Error>` (bytes so CLI can file-or-stdout).

**Messages (in order)**:
1. `FileId` — `r#type=File::WORKOUT`(5); `manufacturer` to a valid non-invalid value (check `typedef::Manufacturer` for a reserved/"none"); rest stay `new()` invalid (skipped).
2. `Workout` — `sport=Sport::SWIMMING`(5), `sub_sport=SubSport::LAP_SWIMMING`(17), `num_valid_steps=flat_steps.len() as u16`, `wkt_name`←name, `wkt_description`←description, `pool_length`/`pool_length_unit`(→`DisplayMeasure::METRIC` 0) from pool; `capabilities` leave 0 (skipped) for v1.
3. one `WorkoutStep` per `Workout::flat_steps()` entry (repeats/IM already expanded; Technique already dropped): `wkt_step_name`←name, `notes`←notes, `intensity` mapped; if `flat.time` Some → `WktStepDuration::TIME`(0)+`duration_value=as_secs()`, no target; else `WktStepDuration::DISTANCE`(1)+`duration_value`=m×100, + stroke target.

**Field facts (source-verified)**: `WorkoutStep.duration_value` is plain `u32` (def `u32::MAX`), no scale helper → we set m×100 (DISTANCE) or secs (TIME). `Workout.pool_length` u16 scale-100 meters (`set_pool_length_scaled` or `=m*100`). `SwimStroke`(=target_value when target_type SWIM_STROKE=11): free=0 back=1 breast=2 fly=3 drill=4 mixed=5 IM=6 im_by_round=7 rimo=8. `Intensity` ACTIVE..OTHER = 0..6. Unset→`new()` invalid→skipped.

**Conversions** (1 yd=0.9144 m; FIT distances are meters×100; pool_length meters×100):
- Distance: meters → `value*100`; yards → `(value*9144+50)/100` (int round).
- pool_length: meters → `length*100`; yards → `(length*9144+50)/100` (25yd→2286, 50m→5000).

**Stroke→target**: Free→(11,0) Back→(11,1) Breast→(11,2) Fly→(11,3) IM→(11,6, defensive); **`Any` (any non-free) → omit target** (FIT has no such code). `stroke=None`: DECIDE omit vs explicit freestyle=0 (lean omit; note choice when coding).

**Decide while coding**: FileId.manufacturer value; None-stroke omit-vs-0; leave `capabilities`=0.

## After encoder
- CLI `generator/src/main.rs` (stub): clap `2fit-gen [--file <path>|-] [-o <out>|stdout]` [+ `--base <mm:ss>` for `@ b`]; notation→parser→to_fit→write.
- Lib exports `generator/src/lib.rs`: re-export `parser`/`fit` + `parse_str`/`to_fit`.
- **Round-trip test**: encode one of the 3 fixtures, `Decoder` it back, assert sport/sub_sport/pool_length + a couple of steps (name/duration/target). Closes the "sample .fit + verify" task.
- Then Scraper (listing+detail+filters) → later the site-format→schema inference.

(Tasks: parser[x] encoder[•] cli/lib[ ] round-trip[ ] scraper[ ] infer[ ])

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
- [ ] Generator: rustyfit encoder core → .fit (Workout + WorkoutStep), file/stdout
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
- Turn 5 (2026-09-01): Implemented the swimdojo parser (`generator/src/parser/swimdojo.rs`, +906). Line classifier (blank → `TOTAL:` → section label → bare/parenthesized subtotal → `N x through:` → `—>` annotation → digit-led step → prose), step builder (counted set, `N x through:` block, drills `bobs`/`sculls` → `Step::Technique`, rest/easy, distance), interval parser (`@ kb`, `@ b`, `@ b +:30` / `+2:00`, fixed `2:00`/`:30`/`45`). Notes attach to the previous step (RepeatStep/RecoveryStep gained `notes` slots). 3 real pages normalized to `documents/swimdojo-fixtures/*.txt`; 12 parser tests pass, each reconciling to its stated total (Goblin 1400, Box Crab 1000, Sea Otter 800). Caught a test-data typo via hexdump: Goblin line 16 was written `@ b +:+30` but the site says `@ b +:30` — fixed the fixture, not the code. Updated `documents/swimdojo-site.md` (confirmed detail URLs + body block classes, subtotal-vs-step-sum caveat, notation-variant list). Workspace green: 30 tests (18 core + 12 generator), `cargo clippy` + `cargo fmt` clean. Commits: `1f3d097` (core Technique), `3e08042` (core notes), `2cea9c5` (parser + fixtures).
- Turn 4 (2026-08-31): User added Guidelines 6–7 (documents/, record knowledge) and updated the Rust toolchain (rustc/cargo now 1.98; rustyfit 0.10.2 requires MSRV 1.93 + edition 2024 — that's why the bump was needed). Bumped workspace rustyfit `0.6.1 → 0.10.2` (latest on crates.io; the lockfile already had 0.10.2), set edition 2024 + `rust-version 1.93`; scaffold builds clean. Implemented core IDL (`core/src/lib.rs`): Unit/Distance/Seconds/Pool/Stroke/Intensity/IntervalSpec/Part/DistanceStep/RepeatStep/BreakdownStep/RecoveryStep/Step/SectionLabel/Section/Workout/FlatStep/Error — 16 unit tests pass (incl. base-math "do not scale the offset", IM breakdown, repeat expansion, serde round-trip). Verified rustyfit encoding API from the registry source → documents/rustyfit.md (key: `Encoder::encode(W: Write+Seek, &mut FIT)`; embedded-io has `Write` for `Vec<u8>` but no `Seek` ⇒ plan a tiny in-memory `Write+Seek` writer; `SubSport::LAP_SWIMMING=17` is the pool swim, not a `POOL` const; `pool_length` scaled ×100).
- Turn 3 (2026-08-30): User added `Changelog` section, `TURN:` field, and Guideline 5 (track turn). Decided FIT crate = **rustyfit v0.10.2** (evidence: encodes + `pool_length` + `DISTANCE`/`SWIM_STROKE` fields). Wrote Plan (workspace layout, core IDL, base-math rule, .fit mapping) and Tasks above. No code yet.
- Turn 1–2: Research only. Explored swimdojo.com (Squarespace) layout + notation; extracted grammar to `/tmp/sd-howto.txt`; surveyed Rust FIT crates; shortlisted rustyfit vs fit-sdk-rust.

