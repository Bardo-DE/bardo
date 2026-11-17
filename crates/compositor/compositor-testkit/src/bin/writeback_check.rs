//! `writeback-check`: the last step of `dev/vkms/accept.sh`'s writeback check.
//!
//! ```text
//! writeback-check RAW W H PITCH FORMAT
//! ```
//!
//! Decodes the raw writeback buffer in `RAW` as a `W` by `H` frame of `PITCH` bytes a row and
//! `FORMAT` pixels (`XR24` or `AR24`), and prints its size. Exit 0 when it decodes, 1 when the
//! buffer is not that frame (too short, a pitch smaller than a row), 64 on bad arguments. It
//! reads one file and writes nothing.

use compositor_testkit::writeback::{RawFrame, decode, format_named};
use std::process::ExitCode;

fn number<T: std::str::FromStr>(text: Option<&String>, what: &str) -> Result<T, String>
where
    T::Err: std::fmt::Display,
{
    text.ok_or_else(|| format!("missing {what}"))?
        .parse()
        .map_err(|error| format!("{what} is not a number: {error}"))
}

fn run(args: &[String]) -> Result<ExitCode, String> {
    let raw = args.first().ok_or("missing RAW")?;
    let width = number(args.get(1), "W")?;
    let height = number(args.get(2), "H")?;
    let pitch = number(args.get(3), "PITCH")?;
    let format = args
        .get(4)
        .and_then(|name| format_named(name))
        .ok_or("FORMAT is XR24 or AR24")?;
    let bytes = std::fs::read(raw).map_err(|error| format!("{raw}: {error}"))?;
    let frame = RawFrame::new(width, height, pitch, format);
    match decode(&frame, &bytes) {
        Ok(image) => {
            println!("writeback-check: {}x{}", image.width(), image.height());
            Ok(ExitCode::SUCCESS)
        }
        Err(error) => {
            println!("writeback-check: {error}");
            Ok(ExitCode::from(1))
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("writeback-check: {error}");
            ExitCode::from(64)
        }
    }
}
