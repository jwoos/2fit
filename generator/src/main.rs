//! `2fit-gen`: read workout text from a file or stdin, write .fit to a file or stdout.

use std::io::{self, Read, Write};
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Parser;
use fit_core::{Pool, Seconds, Unit};
use fit_generator::{fit, parser::swimdojo};

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

    /// Pool length, e.g. `25yd` or `50m` (default `25yd`).
    #[arg(long, default_value = "25yd")]
    pool: String,

    /// Base pace per 100, e.g. `1:40`; required iff the text uses `@ b`.
    #[arg(long)]
    base: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let text = read_input(args.file.as_deref())?;
    let pool = parse_pool(&args.pool)?;
    let base = args.base.as_deref().map(parse_base).transpose()?;
    let workout = swimdojo::parse(&text, pool, base).map_err(|e| anyhow::anyhow!("{e}"))?;
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
    fn bases() {
        assert_eq!(parse_base("1:40").unwrap(), Seconds::secs(100));
        assert_eq!(parse_base(":30").unwrap(), Seconds::secs(30));
        assert_eq!(parse_base("45").unwrap(), Seconds::secs(45));
        assert!(parse_base("1:99").is_err());
        assert!(parse_base("fast").is_err());
    }
}
