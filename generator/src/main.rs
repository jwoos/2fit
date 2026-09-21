//! `2fit-gen`: read workout text from a file or stdin, write .fit to a file or stdout.

use std::io::{self, Read, Write};
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};
use fit_core::{Pool, Seconds, Sport, SubSport, Unit, Workout};
use fit_generator::fit;

/// Target sport (selects FIT sport/sub-sport; run/bike omit the pool).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
enum SportKind {
    /// Pool swimming (default; `--pool` applies).
    #[default]
    Swim,
    /// Running (outdoor/street; `--pool` rejected).
    Run,
    /// Cycling (outdoor/road; `--pool` rejected).
    Bike,
}

impl SportKind {
    fn as_sport(self) -> Sport {
        match self {
            SportKind::Swim => Sport::Swim,
            SportKind::Run => Sport::Run,
            SportKind::Bike => Sport::Bike,
        }
    }
}

/// `--sub-sport` presets (each maps to one [`SubSport`]; cross-sport use
/// falls back to the per-sport default in `effective_sub_sport`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum SubSportKind {
    /// Pool lanes (swim default).
    LapSwim,
    /// Open water.
    OpenWater,
    /// Outdoor road/street running (run default).
    Street,
    /// Trail running / mountain biking.
    Trail,
    /// Track running.
    Track,
    /// Velodrome.
    TrackCycling,
    /// Treadmill running.
    Treadmill,
    /// Indoor running (alias).
    IndoorRun,
    /// Outdoor road cycling (bike default).
    Road,
    /// Mountain biking (alias).
    Mtb,
    /// Indoor cycling.
    Indoor,
    /// Indoor cycling (alias).
    IndoorBike,
    /// No setting (encodes GENERIC).
    Generic,
}

impl SubSportKind {
    fn as_sub_sport(self) -> SubSport {
        match self {
            SubSportKind::LapSwim => SubSport::LapSwim,
            SubSportKind::OpenWater => SubSport::OpenWater,
            SubSportKind::Street => SubSport::Street,
            SubSportKind::Trail | SubSportKind::Mtb => SubSport::Trail,
            SubSportKind::Track => SubSport::Track,
            SubSportKind::TrackCycling => SubSport::Track,
            SubSportKind::Treadmill | SubSportKind::IndoorRun => SubSport::IndoorRun,
            SubSportKind::Road => SubSport::Road,
            SubSportKind::Indoor | SubSportKind::IndoorBike => SubSport::IndoorBike,
            SubSportKind::Generic => SubSport::Generic,
        }
    }
}

/// Parse swimdojo workout notation into a .fit file.
#[derive(Debug, Parser)]
#[command(name = "2fit-gen", version)]
struct Args {
    /// Workout text file (`-` or omitted = stdin).
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Output .fit path (omitted = stdout).
    #[arg(short, long)]
    out: Option<PathBuf>,

    /// Target sport (default `swim`).
    #[arg(long, value_enum, default_value = "swim")]
    sport: SportKind,

    /// Where/how the workout is done: `lap-swim`, `open-water` (swim);
    /// `street` (run default), `trail`, `track`, `treadmill`/`indoor-run`
    /// (run); `road` (bike default), `mtb`/`trail`, `track-cycling`,
    /// `indoor`/`indoor-bike` (bike). Cross-sport values fall back to the
    /// per-sport default rather than encoding wrong.
    #[arg(long, value_enum)]
    sub_sport: Option<SubSportKind>,

    /// Pool length, e.g. `25yd` or `50m` (default `25yd`; swim-only).
    #[arg(long)]
    pool: Option<String>,

    /// Base pace per 100, e.g. `1:40`; required iff the text uses `@ b`.
    #[arg(long)]
    base: Option<String>,

    /// Workout-format schema: a JSON file (see `fit_scraper schema`), or a
    /// built-in preset (`swimdojo`, `myswimpro`, `higdon-run`, `zwift`).
    /// Default follows `--sport`: `swimdojo` for swim, `higdon-run` for
    /// run, `zwift` for bike.
    #[arg(long, value_name = "PATH|PRESET")]
    format: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let text = read_input(args.file.as_deref())?;
    let pool = resolve_pool(args.sport, args.pool.as_deref())?;
    let base = args.base.as_deref().map(parse_base).transpose()?;
    let schema = resolve_schema(args.format.as_deref(), args.sport)?;
    let mut workout = fit_generator::parse_with_schema(&text, pool, base, &schema)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    workout.sport = args.sport.as_sport();
    workout.sub_sport = match args.sub_sport {
        Some(s) => s.as_sub_sport(),
        None => Workout::default_sub_sport(workout.sport),
    };
    if !matches!(args.sport, SportKind::Swim) {
        workout.pool = None;
    }
    let bytes = fit::to_fit(&workout).map_err(|e| anyhow::anyhow!("{e}"))?;
    match args.out.as_deref() {
        Some(path) => {
            std::fs::write(path, &bytes).with_context(|| format!("writing {}", path.display()))?
        }
        None => io::stdout().lock().write_all(&bytes)?,
    }
    Ok(())
}

/// Read the whole workout text from `path` (`None`/`-` = stdin).
fn read_input(path: Option<&std::path::Path>) -> Result<String> {
    match path {
        Some(p) if p.as_os_str() != "-" => {
            std::fs::read_to_string(p).with_context(|| format!("reading {}", p.display()))
        }
        _ => {
            let mut buf = String::new();
            io::stdin()
                .lock()
                .read_to_string(&mut buf)
                .context("reading stdin")?;
            Ok(buf)
        }
    }
}

/// Resolve the parse schema: a JSON file path, a built-in preset name, or
/// the per-sport default (`swimdojo` / `higdon-run` / `zwift`).
fn resolve_schema(format: Option<&str>, sport: SportKind) -> Result<fit_core::FormatSchema> {
    use fit_generator::parser::schema;
    let preset = |name: &str| -> Option<fit_core::FormatSchema> {
        match name {
            "swimdojo" => Some(schema::swimdojo()),
            "myswimpro" => Some(schema::myswimpro()),
            "higdon-run" | "higdon" | "run" => Some(schema::higdon_run()),
            "zwift" | "bike" => Some(schema::zwift()),
            _ => None,
        }
    };
    match format {
        None => Ok(match sport {
            SportKind::Swim => schema::swimdojo(),
            SportKind::Run => schema::higdon_run(),
            SportKind::Bike => schema::zwift(),
        }),
        Some(s) if preset(s).is_some() => Ok(preset(s).unwrap_or_else(schema::swimdojo)),
        Some(path) => {
            let json =
                std::fs::read_to_string(path).with_context(|| format!("reading schema {path}"))?;
            serde_json::from_str(&json).with_context(|| format!("parsing schema {path}"))
        }
    }
}
/// run/bike reject it (`Workout.pool` stays `None`) and parse bare numbers
/// in a throwaway 25-yd pool (unit source only — but note: a run parser in
/// Phase 3 will supply explicit km/mi units instead).
fn resolve_pool(sport: SportKind, pool: Option<&str>) -> Result<Pool> {
    match sport {
        SportKind::Swim => parse_pool(pool.unwrap_or("25yd")),
        SportKind::Run | SportKind::Bike if pool.is_some() => {
            bail!("--pool is swim-only; run/bike workouts carry no pool")
        }
        _ => Ok(Pool::yards25()),
    }
}

/// `<length><unit>`, e.g. `25yd` or `50m`.
fn parse_pool(s: &str) -> Result<Pool> {
    let low = s.trim().to_ascii_lowercase();
    let (unit, num) = if let Some(n) = low
        .strip_suffix("yards")
        .or_else(|| low.strip_suffix("yard"))
        .or_else(|| low.strip_suffix("yds"))
        .or_else(|| low.strip_suffix("yd"))
        .or_else(|| low.strip_suffix('y'))
    {
        (Unit::Yards, n)
    } else if let Some(n) = low
        .strip_suffix("meters")
        .or_else(|| low.strip_suffix("meter"))
        .or_else(|| low.strip_suffix('m'))
    {
        (Unit::Meters, n)
    } else {
        bail!("unparseable pool '{s}': want e.g. 25yd or 50m");
    };
    let length: u32 = num
        .trim()
        .parse()
        .with_context(|| format!("unparseable pool '{s}': want e.g. 25yd or 50m"))?;
    if length == 0 {
        bail!("unparseable pool '{s}': length must be > 0");
    }
    Ok(Pool { length, unit })
}

/// `m:ss`, `:ss`, or bare seconds.
fn parse_base(s: &str) -> Result<Seconds> {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix(':') {
        let secs: u32 = rest
            .parse()
            .with_context(|| format!("unparseable base '{s}': want e.g. 1:40"))?;
        return Ok(Seconds::secs(secs));
    }
    if let Some((m, sec)) = s.split_once(':') {
        let m: u32 = m
            .parse()
            .with_context(|| format!("unparseable base '{s}': want e.g. 1:40"))?;
        let sec: u32 = sec
            .parse()
            .with_context(|| format!("unparseable base '{s}': want e.g. 1:40"))?;
        if sec >= 60 {
            bail!("unparseable base '{s}': seconds must be < 60");
        }
        return Ok(Seconds::secs(m * 60 + sec));
    }
    let secs: u32 = s
        .parse()
        .with_context(|| format!("unparseable base '{s}': want e.g. 1:40"))?;
    Ok(Seconds::secs(secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pools() {
        assert_eq!(
            parse_pool("25yd").unwrap(),
            Pool {
                length: 25,
                unit: Unit::Yards
            }
        );
        assert_eq!(
            parse_pool("50m").unwrap(),
            Pool {
                length: 50,
                unit: Unit::Meters
            }
        );
        assert_eq!(parse_pool("25YD").unwrap().length, 25);
        assert!(parse_pool("pool").is_err());
        assert!(parse_pool("0yd").is_err());
    }

    #[test]
    fn sport_kinds_map() {
        assert_eq!(SportKind::Swim.as_sport(), Sport::Swim);
        assert_eq!(SportKind::Run.as_sport(), Sport::Run);
        assert_eq!(SportKind::Bike.as_sport(), Sport::Bike);
        assert_eq!(SportKind::default(), SportKind::Swim);
    }

    #[test]
    fn pool_resolution() {
        assert_eq!(
            resolve_pool(SportKind::Swim, None).unwrap(),
            Pool::yards25()
        );
        assert_eq!(
            resolve_pool(SportKind::Swim, Some("50m")).unwrap(),
            Pool::meters50()
        );
        assert!(resolve_pool(SportKind::Swim, Some("pool")).is_err());
        assert!(resolve_pool(SportKind::Run, None).is_ok());
        assert!(resolve_pool(SportKind::Run, Some("25yd")).is_err());
        assert!(resolve_pool(SportKind::Bike, Some("25yd")).is_err());
    }

    #[test]
    fn schema_resolution() {
        assert_eq!(
            resolve_schema(None, SportKind::Swim).unwrap().name,
            "swimdojo"
        );
        assert_eq!(
            resolve_schema(None, SportKind::Run).unwrap().name,
            "higdon-run"
        );
        assert_eq!(resolve_schema(None, SportKind::Bike).unwrap().name, "zwift");
        assert_eq!(
            resolve_schema(Some("myswimpro"), SportKind::Swim)
                .unwrap()
                .name,
            "myswimpro"
        );
        assert_eq!(
            resolve_schema(Some("run"), SportKind::Bike).unwrap().name,
            "higdon-run"
        );
        assert!(resolve_schema(Some("/nonexistent.json"), SportKind::Swim).is_err());
    }

    #[test]
    fn sub_sport_kinds_map() {
        assert_eq!(SubSportKind::Street.as_sub_sport(), SubSport::Street);
        assert_eq!(SubSportKind::Treadmill.as_sub_sport(), SubSport::IndoorRun);
        assert_eq!(SubSportKind::Indoor.as_sub_sport(), SubSport::IndoorBike);
        assert_eq!(SubSportKind::Mtb.as_sub_sport(), SubSport::Trail);
        assert_eq!(SubSportKind::Road.as_sub_sport(), SubSport::Road);
    }

    #[test]
    fn bases() {
        assert_eq!(parse_base("1:40").unwrap(), Seconds::secs(100));
        assert_eq!(parse_base(":30").unwrap(), Seconds::secs(30));
        assert_eq!(parse_base("45").unwrap(), Seconds::secs(45));
        assert!(parse_base("1:99").is_err());
        assert!(parse_base("fast").is_err());
    }
}
