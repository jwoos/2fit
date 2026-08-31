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

TURN: 3

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

## Standard format (IDL) — draft
- `Workout { name?, description?, pool: Pool, sections: Vec<Section> }`
- `Pool { length: 25yd | 50m | 25m | 100yd }`
- `Section { label: WarmUp | Main | SetN(n) | WarmDown | None, steps: Vec<Step>, subtotal?: Distance }`
- `Step = Distance{ dist, stroke?, intensity?, interval?: Interval, notes? }
        | Repeat{ n, rest_between?: Secs, inner: Vec<Step> }
        | Breakdown{ dist, parts: Vec<(Distance, Stroke)>, notes? }
        | Rest{ secs }
        | Recovery{ dist }`
- `Interval { base?: Secs, fixed?: Secs, offset?: +Secs | −Secs }` — models `@ :20`, `@ 2:00`, `@ b`, `@ b-5`/`@ b+15`.
  - `b±s` rule: compute the interval for the **longer (full-set) base first, then add/subtract** the offset (100 @ b-5 = 1:55; 200 @ b-5 = 3:55) — do not scale the offset.
- `Stroke { Free, Back, Breast, Fly, IM, Any }` — no stroke ⇒ Free; a lone "stroke" ⇒ non-Freestyle (`Any`).
- IM order fixed Fly→Back→Breast→Free; 100IM=4×25, 200IM=4×50, 300IM=75×3(+Free). Rest on repeats: stated rest between reps; else straight back to start. `50 easy` = active recovery (not on base). (Full grammar: /tmp/sd-howto.txt)

## .fit mapping (core → rustyfit)
- `Workout`: sport=swim, sub_sport=pool, pool_length (25/50), pool_length_unit, wkt_name, wkt_description.
- `WorkoutStep`: duration_type=DISTANCE, duration_value=<distance>, target_type=SWIM_STROKE, target_value=<stroke>, intensity, wkt_step_name, notes.
- `Rest`/`Recovery` → `WorkoutStep` duration_type=TIME, duration_value=<rest secs>.
- `Repeat` → expanded into concrete steps; repetition count drives `num_valid_steps`.

# Tasks
- [x] Research swimdojo layout + notation grammar; extract grammar → /tmp/sd-howto.txt
- [x] Evaluate FIT crates (rustyfit vs fit-sdk-rust) → choose rustyfit v0.10.2
- [ ] Scaffold Cargo workspace (`core`, `generator`, `scraper`) + README
- [ ] Define core IDL types (Workout/Section/Step/Stroke/Interval/Pool, units) + unit tests
- [ ] Generator: swimdojo parser → core (distance, `@` intervals incl. base math, repeats, IM, recovery)
- [ ] Generator: rustyfit encoder core → .fit (Workout + WorkoutStep), file/stdout
- [ ] Generator: CLI (file + stdin) + library exports; parse tests on real swimdojo texts
- [ ] Generate a sample .fit and verify by decoding (round-trip), spot-check fields
- [ ] Scraper: swimdojo scrape (listing + detail) + filters (distance/level/stroke/tag/author/pagination)
- [ ] Scraper: site-format inference → schema (JSON) for the parser (later)

# Bugs/Issues
(none yet)

# References
- swimdojo workouts: https://www.swimdojo.com/workouts (detail: `/workouts/YYYY/M/D/slug`; filters By Distance/Level/Stroke, `?tag=`, `?author=<id>`, `?offset=<epoch-ms>`). Body HTML: `div[data-layout-label="Post Body"]` → `.sqs-block.html-block` → div.sqs-html-content
- swimdojo notation grammar (extracted): `/tmp/sd-howto.txt`
- rustyfit (chosen): https://docs.rs/rustyfit — v0.10.2, encode+decode; WorkoutStep: https://docs.rs/rustyfit/latest/rustyfit/mesgdef/struct.WorkoutStep.html
- fit-sdk-rust (rejected): https://docs.rs/fit-sdk-rust — std crate named `fit`, encodes, younger.

# Changelog
- Turn 3 (2026-08-30): User added `Changelog` section, `TURN:` field, and Guideline 5 (track turn). Decided FIT crate = **rustyfit v0.10.2** (evidence: encodes + `pool_length` + `DISTANCE`/`SWIM_STROKE` fields). Wrote Plan (workspace layout, core IDL, base-math rule, .fit mapping) and Tasks above. No code yet.
- Turn 1–2: Research only. Explored swimdojo.com (Squarespace) layout + notation; extracted grammar to `/tmp/sd-howto.txt`; surveyed Rust FIT crates; shortlisted rustyfit vs fit-sdk-rust.

