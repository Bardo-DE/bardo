//! `bardo-compositorctl`: drives a jailed casement over its private bus.
//!
//! ```text
//! bardo-compositorctl debug inject            lines on stdin, `ok <line>` each (compositor-ipc's grammar)
//! bardo-compositorctl debug wait PREDICATE [--timeout MS]
//! bardo-compositorctl debug clock step N | advance MS
//! bardo-compositorctl debug output list | add SPEC | remove NAME | set NAME KEY=VALUE...
//!                          (or --scale 1.5, --mode WxH@MILLIHERTZ, --position X,Y, --power on|off)
//! bardo-compositorctl output ...              the same, spelt as the freeze plan writes it
//! bardo-compositorctl debug windows | scene
//! bardo-compositorctl debug fault panic-after-frames N | stall-client-ms MS | drop-output INDEX
//! bardo-compositorctl shot [--output NAME | --window ID | --region X,Y,W,H] [--masked] OUT.png
//! bardo-compositorctl report --kms --input --gpu                                              (P5, user only)
//! bardo-compositorctl privacy list | reload        which program may use which power; read the administrator's settings again
//! ```
//!
//! Every subcommand but `report` and `privacy` runs only inside the jail (it refuses otherwise, exit 2).

use compositor_ctl::command;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match command::parse(&args) {
        Ok(command) => command::run(command),
        Err(error) => {
            eprintln!("bardo-compositorctl: {error}");
            ExitCode::from(64)
        }
    }
}
