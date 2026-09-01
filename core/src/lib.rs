//! `fit_core` — the format-independent domain model (the "IDL") shared by the
//! `fit_generator` and `fit_scraper` crates.
//!
//! Types here describe a workout after parsing and before .fit encoding. The
//! full spec, with the evidence behind each decision, lives in
//! `documents/idl.md`.

use serde::{Deserialize, Serialize};
use std::fmt;

// --- Units ------------------------------------------------------------------

/// Measurement unit for distances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Unit {
    /// Yards (imperial pools, e.g. 25-yd US pools).
    Yards,
    /// Meters (metric pools, e.g. 50-m pools).
    Meters,
}

impl Unit {
    /// Short abbreviation used in display strings (`yd`, `m`).
    pub const fn abbreviation(self) -> &'static str {
        match self {
            Unit::Yards => "yd",
            Unit::Meters => "m",
        }
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.abbreviation())
    }
}

/// A distance in a given unit, e.g. 200 meters or 100 yards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Distance {
    /// Numeric distance in `unit`.
    pub value: u32,
    /// Unit the value is expressed in.
    pub unit: Unit,
}

impl Distance {
    /// A distance in yards.
    pub const fn yards(value: u32) -> Self {
        Self {
            value,
            unit: Unit::Yards,
        }
    }

    /// A distance in meters.
    pub const fn meters(value: u32) -> Self {
        Self {
            value,
            unit: Unit::Meters,
        }
    }

    /// Express a pool-unit value as a `Distance` in `pool`'s unit.
    pub const fn in_pool(pool: Pool, value: u32) -> Self {
        Self {
            value,
            unit: pool.unit,
        }
    }
}

impl fmt::Display for Distance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.value, self.unit)
    }
}

// --- Time -------------------------------------------------------------------

/// A time duration in whole seconds. The swimdojo interval notation
/// (`@ 2:00`, `@ :20`) is always parsed into whole seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Seconds(pub u32);

impl Seconds {
    /// Whole seconds.
    pub const fn secs(value: u32) -> Self {
        Self(value)
    }

    /// Whole minutes.
    pub const fn minutes(value: u32) -> Self {
        Self(value * 60)
    }

    /// The duration expressed in seconds.
    pub const fn as_secs(&self) -> u32 {
        self.0
    }

    /// Total as integer seconds, saturating.
    pub const fn saturating_add_secs(&self, other: u32) -> Self {
        Self(self.0.saturating_add(other))
    }

    /// Subtract seconds; `None` when the result would be negative.
    pub fn checked_sub_secs(&self, other: u32) -> Option<Self> {
        self.0.checked_sub(other).map(Self)
    }
}

impl fmt::Display for Seconds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{:02}", self.0 / 60, self.0 % 60)
    }
}

// --- Pool -------------------------------------------------------------------

/// The pool a workout is swum in. Pools in the US are generally 25-yd or
/// 50-m, but any length is representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pool {
    /// Pool length in `unit` (25 for a 25-yd pool, 50 for a 50-m pool).
    pub length: u32,
    /// Unit of the pool length.
    pub unit: Unit,
}

impl Pool {
    /// A 25-yard pool (short course yards).
    pub const fn yards25() -> Self {
        Self {
            length: 25,
            unit: Unit::Yards,
        }
    }

    /// A 25-meter pool (short course meters).
    pub const fn meters25() -> Self {
        Self {
            length: 25,
            unit: Unit::Meters,
        }
    }

    /// A 50-meter pool (long course).
    pub const fn meters50() -> Self {
        Self {
            length: 50,
            unit: Unit::Meters,
        }
    }

    /// A `Distance` in this pool's units.
    pub const fn dist(self, value: u32) -> Distance {
        Distance {
            value,
            unit: self.unit,
        }
    }
}

impl fmt::Display for Pool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{} pool", self.length, self.unit)
    }
}

// --- Strokes ----------------------------------------------------------------

/// A swimming stroke.
///
/// swimdojo convention: no stroke written ⇒ freestyle; a lone "stroke" ⇒
/// any non-freestyle (`Any`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Stroke {
    /// Freestyle.
    Free,
    /// Backstroke.
    Back,
    /// Breaststroke.
    Breast,
    /// Butterfly.
    Fly,
    /// Individual medley, swum in fixed fly → back → breast → free order.
    IM,
    /// Any non-freestyle stroke.
    Any,
}

impl Stroke {
    /// Break an IM distance into its per-stroke parts, in swimdojo order
    /// (fly, back, breast, free).
    ///
    /// Rules (swimdojo "IM" section): 100 IM = 4×25, 200 IM = 4×50,
    /// 300 IM = 4×75. Other IM distances require an explicit breakdown in
    /// the source text and return `None`.
    pub fn im_breakdown(distance: Distance) -> Option<Vec<Part>> {
        let part_len = match distance.value {
            100 => 25,
            200 => 50,
            300 => 75,
            _ => return None,
        };
        Some(vec![
            Part::new(distance.unit_value(part_len), Stroke::Fly),
            Part::new(distance.unit_value(part_len), Stroke::Back),
            Part::new(distance.unit_value(part_len), Stroke::Breast),
            Part::new(distance.unit_value(part_len), Stroke::Free),
        ])
    }
}

impl fmt::Display for Stroke {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Distance {
    /// This distance re-valued to `value`, keeping the same unit
    /// (used by IM breakdowns).
    const fn unit_value(self, value: u32) -> Self {
        Self {
            value,
            unit: self.unit,
        }
    }
}

/// One part of a stroke breakdown, e.g. 25 fly of a 100 IM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Part {
    /// Distance of this part.
    pub distance: Distance,
    /// Stroke for this part.
    pub stroke: Stroke,
}

impl Part {
    /// A new part of `distance` swum as `stroke`.
    pub const fn new(distance: Distance, stroke: Stroke) -> Self {
        Self { distance, stroke }
    }
}

impl fmt::Display for Part {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.distance, self.stroke)
    }
}

// --- Intensity --------------------------------------------------------------

/// Effort level for a step. Mirrors FIT `Intensity` so the .fit mapping is
/// lossless (see `documents/idl.md` § FIT mapping).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Intensity {
    /// Normal working effort (default).
    #[default]
    Active,
    /// Rest.
    Rest,
    /// Warmup.
    Warmup,
    /// Cooldown.
    Cooldown,
    /// Active recovery.
    Recovery,
    /// Interval.
    Interval,
    /// Anything not expressible above.
    Other,
}

impl fmt::Display for Intensity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Intensity::Active => "active",
            Intensity::Rest => "rest",
            Intensity::Warmup => "warmup",
            Intensity::Cooldown => "cooldown",
            Intensity::Recovery => "recovery",
            Intensity::Interval => "interval",
            Intensity::Other => "other",
        })
    }
}

// --- Intervals --------------------------------------------------------------

/// The interval a step is swum on, modeled on swimdojo notation:
///
/// - `@ 2:00` / `@ :20` → [`IntervalSpec::Fixed`]
/// - `@ b` / `@ b-5` / `@ b+15` → [`IntervalSpec::Base`]
///
/// Base math (swimdojo "Bases" section): the offset is applied to the longer
/// (full-set) base, *after* scaling. `100 @ b-5` = 1:55 and `200 @ b-5` =
/// 2×2:00 − 5 = 3:55 — never 2×(2:00 − 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum IntervalSpec {
    /// A fixed interval in seconds, e.g. `@ 2:00`.
    Fixed(Seconds),
    /// Relative to the swimmer's base pace (`base100`) for the step's
    /// distance, plus a signed offset in seconds, e.g. `@ b-5`.
    Base {
        /// Signed offset in seconds (+ faster? no: + slower). `@ b+15`
        /// means 15 seconds slower than base.
        offset: i32,
    },
}

impl IntervalSpec {
    /// Swim exactly on base (`@ b`).
    pub const fn base() -> Self {
        Self::Base { offset: 0 }
    }

    /// A `b±seconds` interval.
    pub const fn base_offset(offset: i32) -> Self {
        Self::Base { offset }
    }

    /// Resolve the interval for `dist` using the swimmer's 100 base.
    ///
    /// `Fixed` ignores `base100`. `Base` scales `base100` proportionally to
    /// the distance in pool units (150 @ b is 1.5× base100) and then
    /// applies the unscaled offset.
    pub fn resolved(self, dist: Distance, base100: Option<Seconds>) -> Result<Seconds, Error> {
        match self {
            IntervalSpec::Fixed(s) => Ok(s),
            IntervalSpec::Base { offset } => {
                let base = base100.ok_or(Error::MissingBase100)?;
                let scaled = base.0 as i64 * dist.value as i64 / 100;
                let total = scaled + i64::from(offset);
                if total < 0 {
                    return Err(Error::NegativeInterval(total));
                }
                Ok(Seconds(total as u32))
            }
        }
    }
}

impl fmt::Display for IntervalSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntervalSpec::Fixed(s) => write!(f, "@ {s}"),
            IntervalSpec::Base { offset } => match offset {
                0 => f.write_str("@ b"),
                o if *o > 0 => write!(f, "@ b+{o}"),
                o => write!(f, "@ {o}"),
            },
        }
    }
}

// --- Steps ------------------------------------------------------------------

/// A single-distance step, e.g. `400 free @ 8:00` or `100`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistanceStep {
    /// Distance of the step.
    pub distance: Distance,
    /// Stroke to swim. `None` means freestyle (swimdojo default).
    pub stroke: Option<Stroke>,
    /// Interval to swim on, if any.
    pub interval: Option<IntervalSpec>,
    /// Effort level of the step.
    #[serde(default)]
    pub intensity: Intensity,
    /// Free-form notes (breathing patterns, form cues, ...).
    pub notes: Option<String>,
}

/// A repeated sequence of steps, e.g. `2x through: 4 x 100 @ 2:00 ...`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepeatStep {
    /// Number of times to repeat `inner`.
    pub count: u32,
    /// Rest inserted after each repetition except the last, if any.
    pub rest_between: Option<Seconds>,
    /// The steps repeated, in order.
    pub inner: Vec<Step>,
    /// Free-form notes applying to the whole repetition, e.g. per-rep-range
    /// splits (`#1-3 kick, #4-6 pull`).
    #[serde(default)]
    pub notes: Option<String>,
}

/// A stroke breakdown of one distance, e.g. the parts of `100 IM`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BreakdownStep {
    /// Total distance covered by all `parts`.
    pub distance: Distance,
    /// The parts in swum order.
    pub parts: Vec<Part>,
    /// Effort level of the step.
    #[serde(default)]
    pub intensity: Intensity,
    /// Free-form notes.
    pub notes: Option<String>,
}

/// Active-recovery step swum off base, e.g. `50 easy`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryStep {
    /// Distance of the recovery swim.
    pub distance: Distance,
    /// Recovery stroke; `None` means freestyle.
    pub stroke: Option<Stroke>,
    /// Free-form notes.
    #[serde(default)]
    pub notes: Option<String>,
}

/// A technique drill counted by reps rather than by swim distance or rest
/// time, e.g. `3 x 10 bobs @ :30` (three sets of ten bobs, 30 s between
/// sets).
///
/// The leading number of such a line is a rep count, not a pool distance —
/// counting it would inflate the workout total (e.g. swimdojo's Sea Otter
/// states `TOTAL: 800` for a warm-up containing `3 x 10 bobs`, which only
/// holds if the bobs add 0 distance). Distance drills such as `200 kick`
/// are *not* [`Step::Technique`]: there the number is a distance, so they
/// parse as [`Step::Distance`].
///
/// Technique steps contribute [`Step::distance_value()`] = 0 and are
/// skipped by [`Workout::flat_steps`] (v1 does not emit them as .fit
/// workout steps).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TechniqueStep {
    /// The drill or cue, e.g. "bobs".
    pub drill: String,
    /// Reps per set — the leading number (`10` in `3 x 10 bobs`).
    pub reps_per_set: Option<u32>,
    /// Number of sets — a leading `N x` prefix (`3` in `3 x 10 bobs`).
    pub sets: Option<u32>,
    /// Interval between sets, if stated (`@ :30`).
    pub interval: Option<IntervalSpec>,
    /// Free-form notes.
    pub notes: Option<String>,
}

impl TechniqueStep {
    /// Total reps across all sets (`sets * reps_per_set`, unknown factors
    /// treated as 1).
    pub fn total_reps(&self) -> u32 {
        let sets = self.sets.unwrap_or(1);
        let reps = self.reps_per_set.unwrap_or(1);
        sets * reps
    }
}

/// One line of a workout, in any format that can be mapped to the IDL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Step {
    /// A distance swum (possibly with stroke and interval).
    Distance(DistanceStep),
    /// A sequence repeated a number of times.
    Repeat(RepeatStep),
    /// A distance split across strokes.
    Breakdown(BreakdownStep),
    /// A rest between sets, e.g. `30 seconds rest`.
    Rest {
        /// Rest duration.
        secs: Seconds,
    },
    /// Active recovery swum off base, e.g. `50 easy`.
    Recovery(RecoveryStep),
    /// A technique drill counted by reps, e.g. `3 x 10 bobs`.
    Technique(TechniqueStep),
}

impl Step {
    /// Total distance this step contributes, in the step's own units
    /// (0 for [`Step::Rest`]).
    pub fn distance_value(&self) -> u32 {
        match self {
            Step::Distance(s) => s.distance.value,
            Step::Repeat(r) => r.count * r.inner.iter().map(|s| s.distance_value()).sum::<u32>(),
            Step::Breakdown(b) => b.parts.iter().map(|p| p.distance.value).sum(),
            Step::Rest { .. } => 0,
            Step::Recovery(r) => r.distance.value,
            Step::Technique(_) => 0,
        }
    }

    /// A compact, human-readable label (e.g. `4 x (100 @ 2:00)`). Used for
    /// display and as the FIT `wkt_step_name`; it is *not* a canonical
    /// round-trip format.
    #[must_use]
    pub fn display(&self) -> String {
        match self {
            Step::Distance(s) => {
                let mut out = s.distance.to_string();
                if let Some(st) = s.stroke
                    && st != Stroke::Free
                {
                    out.push(' ');
                    out.push_str(st.as_str());
                }
                if let Some(iv) = &s.interval {
                    out.push(' ');
                    out.push_str(&iv.to_string());
                }
                out
            }
            Step::Repeat(r) => {
                let inner = r
                    .inner
                    .iter()
                    .map(Step::display)
                    .collect::<Vec<_>>()
                    .join(", ");
                let mut out = format!("{} x ({inner})", r.count);
                if let Some(rest) = r.rest_between {
                    out.push_str(&format!(", {rest} rest between"));
                }
                out
            }
            Step::Breakdown(b) => {
                let parts = b
                    .parts
                    .iter()
                    .map(Part::to_string)
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("{}: ({parts})", b.distance)
            }
            Step::Rest { secs } => format!("{secs} rest"),
            Step::Recovery(r) => format!("{} easy", r.distance),
            Step::Technique(t) => {
                let mut out = String::new();
                if let Some(sets) = t.sets {
                    out.push_str(&format!("{sets} x "));
                }
                if let Some(reps) = t.reps_per_set {
                    out.push_str(&format!("{reps} "));
                }
                out.push_str(&t.drill);
                if let Some(iv) = &t.interval {
                    out.push(' ');
                    out.push_str(&iv.to_string());
                }
                out
            }
        }
    }
}

impl Stroke {
    /// The short name as it appears in display strings.
    pub const fn as_str(self) -> &'static str {
        match self {
            Stroke::Free => "free",
            Stroke::Back => "back",
            Stroke::Breast => "breast",
            Stroke::Fly => "fly",
            Stroke::IM => "IM",
            Stroke::Any => "stroke",
        }
    }
}

// --- Sections ---------------------------------------------------------------

/// Label of a workout section (swimdojo: Warm Up / Main / Set N / Cool Down).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum SectionLabel {
    /// No label.
    #[default]
    None,
    /// Warmup.
    WarmUp,
    /// The main set.
    Main,
    /// A numbered set.
    Set(u32),
    /// Cool-down / warm-down.
    CoolDown,
}

impl fmt::Display for SectionLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SectionLabel::None => Ok(()),
            SectionLabel::WarmUp => f.write_str("Warm Up"),
            SectionLabel::Main => f.write_str("Main Set"),
            SectionLabel::Set(n) => write!(f, "Set {n}"),
            SectionLabel::CoolDown => f.write_str("Warm Down"),
        }
    }
}

/// A labeled group of steps, e.g. the Warm Up.
///
/// A workout is a sequence of sections; a section is a sequence of steps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    /// Label of the section.
    pub label: SectionLabel,
    /// Steps in the section, in order.
    pub steps: Vec<Step>,
    /// The subtotal the source states for this section, if it states one
    /// (swimdojo prints it in italics under each set).
    pub subtotal: Option<Distance>,
}

impl Section {
    /// Total distance of the section's steps, in the section's units.
    pub fn total_distance(&self) -> u32 {
        self.steps.iter().map(Step::distance_value).sum()
    }
}

// --- Workout ----------------------------------------------------------------

/// A complete, parsed workout: metadata + pool + ordered sections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workout {
    /// Workout name, if the source has one.
    pub name: Option<String>,
    /// Free-form description, if the source has one.
    pub description: Option<String>,
    /// The pool the workout is written for.
    pub pool: Pool,
    /// The swimmer's base pace per 100 (pool units), if known. Required to
    /// resolve [`IntervalSpec::Base`]; see swimdojo "Bases".
    pub base100: Option<Seconds>,
    /// The workout's sections, in swum order.
    pub sections: Vec<Section>,
}

/// A step with repeats/breakdowns already expanded, in the exact order it
/// would be swum. This is what the .fit encoder consumes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlatStep {
    /// Set for swim steps.
    pub distance: Option<Distance>,
    /// Set for rest steps.
    pub time: Option<Seconds>,
    /// Stroke, if known.
    pub stroke: Option<Stroke>,
    /// Human-readable label (FIT `wkt_step_name`).
    pub name: Option<String>,
    /// Free-form notes (FIT `notes`).
    pub notes: Option<String>,
    /// Effort level.
    #[serde(default)]
    pub intensity: Intensity,
}

impl Workout {
    /// A new workout for `pool` with no sections.
    pub const fn new(pool: Pool) -> Self {
        Self {
            name: None,
            description: None,
            pool,
            base100: None,
            sections: Vec::new(),
        }
    }

    /// A `Distance` in this workout's pool units.
    pub const fn dist(&self, value: u32) -> Distance {
        Distance::in_pool(self.pool, value)
    }

    /// Total distance of the whole workout, in pool units.
    pub fn total_distance(&self) -> Distance {
        Distance {
            value: self.sections.iter().map(Section::total_distance).sum(),
            unit: self.pool.unit,
        }
    }

    /// Expand all repeats and IMs into the ordered list of steps to swim.
    ///
    /// `IntervalSpec::Base` values are *not* resolved here (they need
    /// `base100`); the caller resolves them via
    /// [`IntervalSpec::resolved`] when encoding.
    pub fn flat_steps(&self) -> Vec<FlatStep> {
        let mut out = Vec::new();
        for section in &self.sections {
            for step in &section.steps {
                self.flatten_step(step, &mut out);
            }
        }
        out
    }

    fn flatten_step(&self, step: &Step, out: &mut Vec<FlatStep>) {
        match step {
            Step::Distance(s) => {
                if let Some(Stroke::IM) = s.stroke
                    && let Some(parts) = Stroke::im_breakdown(s.distance)
                {
                    for p in parts {
                        out.push(FlatStep {
                            distance: Some(p.distance),
                            time: None,
                            stroke: Some(p.stroke),
                            name: Some(p.to_string()),
                            notes: s.notes.clone(),
                            intensity: s.intensity,
                        });
                    }
                    return;
                }
                out.push(FlatStep {
                    distance: Some(s.distance),
                    time: None,
                    stroke: s.stroke,
                    name: Some(step.display()),
                    notes: s.notes.clone(),
                    intensity: s.intensity,
                });
            }
            Step::Repeat(r) => {
                let start = out.len();
                for rep in 0..r.count {
                    for inner in &r.inner {
                        self.flatten_step(inner, out);
                    }
                    if let Some(rest) = r.rest_between
                        && rep + 1 < r.count
                    {
                        out.push(FlatStep {
                            distance: None,
                            time: Some(rest),
                            stroke: None,
                            name: Some(format!("{rest} rest")),
                            notes: None,
                            intensity: Intensity::Rest,
                        });
                    }
                }
                // Whole-repeat notes (e.g. per-rep-range splits) apply to the
                // first step swum in the first repetition.
                if let Some(n) = &r.notes
                    && out.len() > start
                {
                    let first = &mut out[start];
                    first.notes = Some(match first.notes.take() {
                        Some(p) => format!("{n} {p}"),
                        None => n.clone(),
                    });
                }
            }
            Step::Breakdown(b) => {
                for p in &b.parts {
                    out.push(FlatStep {
                        distance: Some(p.distance),
                        time: None,
                        stroke: Some(p.stroke),
                        name: Some(p.to_string()),
                        notes: b.notes.clone(),
                        intensity: b.intensity,
                    });
                }
            }
            Step::Rest { secs } => {
                out.push(FlatStep {
                    distance: None,
                    time: Some(*secs),
                    stroke: None,
                    name: Some(format!("{secs} rest")),
                    notes: None,
                    intensity: Intensity::Rest,
                });
            }
            Step::Recovery(r) => {
                out.push(FlatStep {
                    distance: Some(r.distance),
                    time: None,
                    stroke: r.stroke,
                    name: Some(format!("{} easy", r.distance)),
                    notes: r.notes.clone(),
                    intensity: Intensity::Recovery,
                });
            }
            // Technique drills (rep counted, not distance) are not emitted as
            // .fit steps in v1 (see `TechniqueStep` docs).
            Step::Technique(_) => {}
        }
    }

    /// Number of FIT `WorkoutStep` messages the encoder will emit
    /// (= number of flat steps).
    pub fn num_valid_steps(&self) -> usize {
        self.flat_steps().len()
    }
}

impl fmt::Display for Workout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = &self.name {
            writeln!(f, "{name}")?;
        }
        for section in &self.sections {
            if !section.label.to_string().is_empty() {
                writeln!(f, "{}", section.label)?;
            }
            for step in &section.steps {
                writeln!(f, "  {}", step.display())?;
            }
            if let Some(sub) = section.subtotal {
                writeln!(f, "  {sub}")?;
            }
        }
        Ok(())
    }
}

// --- Errors -----------------------------------------------------------------

/// Errors from the IDL itself (interval resolution).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// A `@ b` interval was used but the workout has no `base100`.
    MissingBase100,
    /// A `@ b` interval resolved to a negative time (seconds).
    NegativeInterval(i64),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MissingBase100 => {
                f.write_str("interval uses base (@ b) but no base100 was provided")
            }
            Error::NegativeInterval(total) => {
                write!(f, "interval resolved to negative time ({total}s)")
            }
        }
    }
}

impl std::error::Error for Error {}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn base100() -> Seconds {
        Seconds::minutes(2)
    }

    #[test]
    fn seconds_display() {
        assert_eq!(Seconds::secs(0).to_string(), "0:00");
        assert_eq!(Seconds::secs(20).to_string(), "0:20");
        assert_eq!(Seconds::secs(115).to_string(), "1:55");
        assert_eq!(Seconds::minutes(10).to_string(), "10:00");
    }

    #[test]
    fn seconds_arithmetic() {
        assert_eq!(base100().checked_sub_secs(5), Some(Seconds::secs(115)));
        assert_eq!(base100().saturating_add_secs(35), Seconds::secs(155));
        assert_eq!(Seconds::secs(5).checked_sub_secs(10), None);
    }

    #[test]
    fn distance_and_pool_display() {
        assert_eq!(Distance::meters(200).to_string(), "200 m");
        assert_eq!(Distance::yards(100).to_string(), "100 yd");
        let pool = Pool::meters50();
        assert_eq!(pool.dist(400), Distance::meters(400));
        assert_eq!(pool.to_string(), "50-m pool");
    }

    #[test]
    fn interval_fixed_ignores_base() {
        assert_eq!(
            IntervalSpec::Fixed(Seconds::secs(120))
                .resolved(Distance::yards(100), Some(base100()))
                .unwrap(),
            Seconds::secs(120)
        );
    }

    #[test]
    fn interval_base_on_100() {
        let dist = Distance::yards(100);
        assert_eq!(
            IntervalSpec::base_offset(-5)
                .resolved(dist, Some(base100()))
                .unwrap(),
            Seconds::secs(115)
        );
        assert_eq!(
            IntervalSpec::base_offset(15)
                .resolved(dist, Some(base100()))
                .unwrap(),
            Seconds::secs(135)
        );
    }

    #[test]
    fn interval_base_offset_is_not_scaled() {
        // swimdojo: 200 @ b-5 = 3:55 (NOT 2×(2:00−5) = 3:50).
        let dist = Distance::yards(200);
        assert_eq!(
            IntervalSpec::base_offset(-5)
                .resolved(dist, Some(base100()))
                .unwrap(),
            Seconds::secs(235)
        );
        assert_eq!(
            IntervalSpec::base_offset(5)
                .resolved(dist, Some(base100()))
                .unwrap(),
            Seconds::secs(245)
        );
    }

    #[test]
    fn interval_base_scales_proportionally_for_sub_100() {
        assert_eq!(
            IntervalSpec::base()
                .resolved(Distance::yards(75), Some(base100()))
                .unwrap(),
            Seconds::secs(90)
        );
        assert_eq!(
            IntervalSpec::base()
                .resolved(Distance::yards(150), Some(base100()))
                .unwrap(),
            Seconds::secs(180)
        );
    }

    #[test]
    fn interval_base_requires_base100() {
        assert_eq!(
            IntervalSpec::base().resolved(Distance::yards(100), None),
            Err(Error::MissingBase100)
        );
    }

    #[test]
    fn interval_base_cannot_be_negative() {
        assert_eq!(
            IntervalSpec::base_offset(-300).resolved(Distance::yards(100), Some(base100())),
            Err(Error::NegativeInterval(-180))
        );
    }

    #[test]
    fn im_breakdown() {
        let d100 = Distance::yards(100);
        let parts = Stroke::im_breakdown(d100).unwrap();
        assert_eq!(
            parts,
            vec![
                Part::new(Distance::yards(25), Stroke::Fly),
                Part::new(Distance::yards(25), Stroke::Back),
                Part::new(Distance::yards(25), Stroke::Breast),
                Part::new(Distance::yards(25), Stroke::Free),
            ]
        );
        assert!(Stroke::im_breakdown(Distance::yards(50)).is_none());
        assert_eq!(
            Stroke::im_breakdown(Distance::yards(200))
                .unwrap()
                .iter()
                .map(|p| p.distance)
                .collect::<Vec<_>>(),
            vec![Distance::yards(50); 4]
        );
        assert_eq!(
            Stroke::im_breakdown(Distance::yards(300))
                .unwrap()
                .iter()
                .map(|p| p.distance)
                .collect::<Vec<_>>(),
            vec![Distance::yards(75); 4]
        );
    }

    /// `2x through: 4 x 100 @ 2:00, 6 x 50 @ 1:00, 30 seconds rest`
    fn repeat_workout() -> Workout {
        let pool = Pool::yards25();
        let mut w = Workout::new(pool);
        w.base100 = Some(base100());
        let set = Step::Repeat(RepeatStep {
            count: 2,
            rest_between: None,
            inner: vec![
                Step::Repeat(RepeatStep {
                    count: 4,
                    rest_between: None,
                    inner: vec![Step::Distance(DistanceStep {
                        distance: Distance::yards(100),
                        stroke: None,
                        interval: Some(IntervalSpec::Fixed(Seconds::minutes(2))),
                        intensity: Intensity::default(),
                        notes: None,
                    })],
                    notes: None,
                }),
                Step::Repeat(RepeatStep {
                    count: 6,
                    rest_between: None,
                    inner: vec![Step::Distance(DistanceStep {
                        distance: Distance::yards(50),
                        stroke: None,
                        interval: Some(IntervalSpec::Fixed(Seconds::minutes(1))),
                        intensity: Intensity::default(),
                        notes: None,
                    })],
                    notes: None,
                }),
                Step::Rest {
                    secs: Seconds::secs(30),
                },
            ],
            notes: None,
        });
        w.sections.push(Section {
            label: SectionLabel::Main,
            steps: vec![set],
            subtotal: Some(Distance::yards(1400)),
        });
        w
    }

    #[test]
    fn flat_steps_expand_repeats() {
        let w = repeat_workout();
        let flat = w.flat_steps();
        // 2 throughs × (4×100 + 6×50 + 30 rest) = 2 × 11 = 22 steps.
        assert_eq!(flat.len(), 22);
        assert_eq!(w.num_valid_steps(), 22);
        assert_eq!(flat[0].distance, Some(Distance::yards(100)));
        assert_eq!(flat[0].stroke, None);
        assert_eq!(flat[4].distance, Some(Distance::yards(50)));
        assert_eq!(flat[10].time, Some(Seconds::secs(30)));
        assert_eq!(flat[11].distance, Some(Distance::yards(100)));
    }

    #[test]
    fn repeat_rest_between_is_not_appended_after_last_rep() {
        let mut w = Workout::new(Pool::yards25());
        w.sections.push(Section {
            label: SectionLabel::Main,
            steps: vec![Step::Repeat(RepeatStep {
                count: 3,
                rest_between: Some(Seconds::secs(10)),
                inner: vec![Step::Distance(DistanceStep {
                    distance: Distance::yards(100),
                    stroke: None,
                    interval: None,
                    intensity: Intensity::default(),
                    notes: None,
                })],
                notes: None,
            })],
            subtotal: None,
        });
        // 3 swims + rest after reps 1 and 2 only.
        assert_eq!(w.flat_steps().len(), 5);
    }

    #[test]
    fn repeat_notes_carry_to_first_inner_step() {
        let mut w = Workout::new(Pool::yards25());
        w.sections.push(Section {
            label: SectionLabel::Main,
            steps: vec![Step::Repeat(RepeatStep {
                count: 2,
                rest_between: None,
                inner: vec![Step::Distance(DistanceStep {
                    distance: Distance::yards(100),
                    stroke: None,
                    interval: None,
                    intensity: Intensity::default(),
                    notes: None,
                })],
                notes: Some("#1-3 kick, #4-6 pull".into()),
            })],
            subtotal: None,
        });
        let flat = w.flat_steps();
        assert_eq!(flat.len(), 2);
        assert_eq!(flat[0].notes.as_deref(), Some("#1-3 kick, #4-6 pull"));
        assert_eq!(flat[1].notes, None);
    }

    #[test]
    fn flat_steps_expand_im() {
        let mut w = Workout::new(Pool::yards25());
        w.sections.push(Section {
            label: SectionLabel::Main,
            steps: vec![Step::Distance(DistanceStep {
                distance: Distance::yards(100),
                stroke: Some(Stroke::IM),
                interval: None,
                intensity: Intensity::default(),
                notes: None,
            })],
            subtotal: None,
        });
        let flat = w.flat_steps();
        assert_eq!(flat.len(), 4);
        assert_eq!(
            flat.iter().map(|s| s.stroke).collect::<Vec<_>>(),
            vec![
                Some(Stroke::Fly),
                Some(Stroke::Back),
                Some(Stroke::Breast),
                Some(Stroke::Free),
            ]
        );
        assert_eq!(
            flat.iter().map(|s| s.distance).collect::<Vec<_>>(),
            vec![Some(Distance::yards(25)); 4]
        );
    }

    #[test]
    fn technique_step_adds_no_distance_and_is_flattened_out() {
        let mut w = Workout::new(Pool::yards25());
        let bobs = TechniqueStep {
            drill: "bobs".into(),
            reps_per_set: Some(10),
            sets: Some(3),
            interval: Some(IntervalSpec::Fixed(Seconds::secs(30))),
            notes: None,
        };
        assert_eq!(bobs.total_reps(), 30);
        assert_eq!(Step::Technique(bobs.clone()).distance_value(), 0);
        w.sections.push(Section {
            label: SectionLabel::WarmUp,
            steps: vec![
                Step::Distance(DistanceStep {
                    distance: Distance::yards(100),
                    stroke: None,
                    interval: None,
                    intensity: Intensity::default(),
                    notes: None,
                }),
                Step::Technique(bobs),
            ],
            subtotal: None,
        });
        // 100 swum; the 3 x 10 bobs add 0 distance and 0 flat steps.
        assert_eq!(w.total_distance(), Distance::yards(100));
        let flat = w.flat_steps();
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].distance, Some(Distance::yards(100)));
        assert_eq!(
            Step::Technique(TechniqueStep {
                drill: "bobs".into(),
                reps_per_set: Some(10),
                sets: Some(3),
                interval: Some(IntervalSpec::Fixed(Seconds::secs(30))),
                notes: None,
            })
            .display(),
            "3 x 10 bobs @ 0:30"
        );
    }

    #[test]
    fn total_distance_sums_sections_and_repeats() {
        let w = repeat_workout();
        // 2 throughs × (4×100 + 6×50) = 1400.
        assert_eq!(w.total_distance(), Distance::yards(1400));
    }

    #[test]
    fn workout_display_contains_steps() {
        let w = repeat_workout();
        let s = w.to_string();
        assert!(s.contains("Main Set"));
        assert!(s.contains("1400 yd"));
        assert!(s.contains("100 yd @ 2:00"));
    }

    #[test]
    fn serde_round_trip() {
        let w = repeat_workout();
        let json = serde_json::to_string(&w).unwrap();
        let back: Workout = serde_json::from_str(&json).unwrap();
        assert_eq!(w, back);
    }
}
