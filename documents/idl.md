# IDL — `fit_core` domain model

The format-independent workout model shared by `fit_generator` and
`fit_scraper`. Types live in `core/src/lib.rs`; this document is the spec,
with the evidence behind each decision (Guideline 4). Grammar source:
`documents/swimdojo-grammar.txt` (swimdojo "How To").

## Types

| Type | Shape | Notes |
| --- | --- | --- |
| `Unit` | `Yards \| Meters \| Kilometers \| Miles` | km/mi from Higdon run survey (`run-bike-notation.md` §1). |
| `Distance` | `{ value: u32, unit: Unit }` | e.g. 200 m, 100 yd, 5 km, 3 mi. |
| `Seconds(u32)` | whole seconds | displays `m:ss`. swimdojo `@ 2:00` / `@ :20` → 120 / 20. |
| `Sport` | `Swim* \| Run \| Bike` | default Swim preserves v1 behavior. |
| `Pool` | `{ length: u32, unit: Unit }` | consts `yards25()`, `meters25()`, `meters50()`; any length representable. |
| `Stroke` | `Free \| Back \| Breast \| Fly \| IM \| Any` | no-stroke ⇒ Free; lone "stroke" ⇒ Any (grammar § Strokes). |
| `Pace` | `Seconds` per km | displays `m:ss/km`; encodes to FIT speed m/s × 1000. |
| `Target` | `Swim(Stroke) \| Pace(Pace) \| Power(u32 watts) \| HrZone(u8) \| Cadence(u8 rpm)` | `None` = no target (Higdon base format). `Swim` replaces `stroke: Option<Stroke>`. |
| `Intensity` | `Active* \| Rest \| Warmup \| Cooldown \| Recovery \| Interval \| Other` | mirrors FIT `Intensity` (0..6) 1:1 so .fit mapping is lossless. |
| `IntervalSpec` | `Fixed(Seconds) \| Base { offset: i32 }` | `@ 2:00` → Fixed; `@ b`, `@ b-5`, `@ b+15` → Base. |
| `Part` | `{ distance, stroke }` | one part of a breakdown. |
| `DistanceStep` | `{ distance, stroke?, interval?, intensity, notes? }` | stroke `None` = freestyle. |
| `TimedStep` | `{ duration, target?, intensity, notes? }` | time-based work (`2 x 20:00 @ 250W`); run/bike. |
| `RepeatStep` | `{ count, rest_between?, inner: Vec<Step> }` | "2x through: …". `rest_between` is inserted after every rep **except the last**. |
| `BreakdownStep` | `{ distance, parts, intensity, notes? }` | explicit breakdowns (e.g. 75 IMs with custom splits). |
| `RecoveryStep` | `{ distance, stroke? }` | "50 easy" — active recovery, off base (grammar § Easy swims). |
| `Step` | `Distance \| Timed \| Repeat \| Breakdown \| Rest { secs } \| Recovery` | one workout line. `distance_value()` = 0 for Rest/Timed (rests don't count toward swum distance; timed work has no distance). |
| `SectionLabel` | `None* \| WarmUp \| Main \| Set(u32) \| CoolDown` | swimdojo prints "Warm Up", "Set N", italic subtotals. |
| `Section` | `{ label, steps, subtotal? }` | subtotal = the italic set total if the source states one. |
| `Workout` | `{ name?, description?, sport=Swim, pool?, base100?, sections }` | `base100` = swimmer's pace per 100 **in pool units**, if known. `pool: None` for run/bike (encoder omits `pool_length`). |
| `FlatStep` | `{ distance?, time?, stroke?, target?, name?, notes?, intensity }` | repeat/breakdowns expanded, in swum order — what the .fit encoder consumes. `FlatStep.target` parallels `stroke` (swim flattening sets both; timed sets `target` only). |

All types derive `Serialize`/`Deserialize` (schema export + scraper output later).

## Key rules (evidence in grammar)

1. **FIT has no repeat primitive.** Repeats/breakdowns are expanded:
   `Workout::flat_steps()`; `Workout::num_valid_steps()` = len — this drives
   FIT `workout.num_valid_steps`. (Garmin FIT spec: `num_valid_steps` =
   "number of valid steps" following the workout message.)
2. **Base math** (grammar § Bases): offset applies to the *full-distance*
   base, computed *before* the offset: `200 @ b-5` = 2×base100 − 5 = 3:55,
   **not** 2×(base100 − 5). Sub-100 distances scale proportionally
   (75 @ b = 1.5×base100) — swimdojo itself defers to a "Pacing Table" we
   don't have, so proportional scaling is the documented assumption.
   `IntervalSpec::resolved(dist, base100)`; `None` base100 + `@ b` ⇒
   `Error::MissingBase100` (parser must surface it, not guess).
3. **IM order is fixed**: fly → back → breast → free (grammar § IM).
   100 IM = 4×25, 200 IM = 4×50, 300 IM = 4×75 (swimdojo: "a 300 is 75 of
   each"); other IM distances require an explicit written breakdown ⇒
   `Stroke::im_breakdown` returns `None`, parser keeps the raw IM step.
4. **Rest semantics**: a stated rest line (grammar: "30 seconds rest") is a
   `Rest` step at the *end* of the repeated block; no rest ⇒ "go directly
   back into the first part of the set" (grammar § Repeats).
5. **`50 easy`** is `Recovery` (off base), not a `DistanceStep` (grammar §
   Easy swims: "not on any sort of base").

## FIT mapping (for `fit_generator::fit`)

- `Workout` → FIT `Workout`: sport/run/bike per `Workout.sport`
  (swim=5/lap_swimming=17, run=1/street=2, bike=2/road=7 — rustyfit
  `profile/typedef/{sport,sub_sport}.rs`), `pool_length` = pool.length × 100
  (scale 100, unit m per Profile.xlsx — the value is in `pool_length_unit`
  units; omitted when `pool: None`), `pool_length_unit` =
  display_measure metric(0)/statute(1) (km→metric, mi→statute), wkt_name,
  wkt_description, num_valid_steps = flat step count.
- `FlatStep.distance` → `WorkoutStep` duration_type=DISTANCE(1),
  duration_value = meters×100 (scale 100) for metric, yards×100 for statute,
  km×100_000, mi×160_934.4 (integer round),
  target_type=SWIM_STROKE(11), target_value = stroke (FIT swim-stroke codes,
  to confirm in rustfit typedefs).
- `FlatStep.time` (rest or `Timed` work) → duration_type=TIME(0),
  duration_value = seconds, intensity=REST(1) for rests.
- `FlatStep.target` → `target_type`/`target_value`: Pace→SPEED(0) as m/s×1000
  (`1_000_000 / secs_per_km`), Power→POWER(4) watts, HrZone→HEART_RATE(1)
  zone number, Cadence→CADENCE(3) rpm, Swim→SWIM_STROKE(11). Applies to both
  distance and timed steps. `target` wins over legacy `stroke`; both unset
  (or `Any`) omits the target.
- Full verified rustyfit API: `documents/rustyfit.md`.

## Deviations from earlier plan draft

- `Interval` in the plan (`{ base?, fixed?, offset? }`) split into
  `IntervalSpec::Fixed | Base { offset }` + `Workout.base100`: bases are
  *swimmer* data, never written in the workout text (grammar § Bases shows
  no base value in notation), so they don't belong inside an interval spec.
- Step variants are structs instead of inline fields (extensibility per
  Guideline 3; serde stability for future schema work).
