//! `cargo xtask install`: builds and installs the desktop with the packaging scripts.
//!
//! The scripts do the work; this only maps the arguments to one of them and runs it in
//! `<workspace root>/packaging`. The part list lives in `install/plain.sh` (its `parts` mode).

use std::process::Command;

use anyhow::{Result, bail};

use super::{finish, run_logged};
use crate::workspace::Workspace;

/// What was asked on the command line.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Request {
    /// `--build`: build as you, install nothing.
    pub build: bool,
    /// `--check`: report what this machine lacks.
    pub check: bool,
    /// `--parts`: list the parts in install order.
    pub parts: bool,
    /// The parts named, or every part when empty.
    pub names: Vec<String>,
}

/// The script to run: a program and its arguments, relative to the `packaging` folder.
#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    pub program: &'static str,
    pub args: Vec<String>,
}

/// Maps a request to the script that serves it, or says why the flags do not go together.
pub fn plan(request: &Request) -> Result<Plan> {
    let flags = [request.build, request.check, request.parts]
        .into_iter()
        .filter(|&on| on)
        .count();
    if flags > 1 {
        bail!("--build, --check and --parts cannot be combined");
    }
    if (request.check || request.parts) && !request.names.is_empty() {
        bail!("--check and --parts take no part names");
    }
    let plain = |mode: &str| {
        let mut args = vec![mode.to_owned()];
        args.extend(request.names.iter().cloned());
        Plan {
            program: "install/plain.sh",
            args,
        }
    };
    Ok(if request.check {
        Plan {
            program: "./install.sh",
            args: vec!["--check".to_owned()],
        }
    } else if request.parts {
        plain("parts")
    } else if request.build {
        plain("build")
    } else {
        plain("install")
    })
}

/// Runs the chosen script in `packaging/`. PREFIX comes from the environment (the scripts default it to /usr/local).
pub fn run(request: &Request) -> Result<bool> {
    let plan = plan(request)?;
    let ws = Workspace::load()?;
    let green = run_logged(
        &ws.root.join("packaging"),
        Command::new(plan.program).args(&plan.args),
    )?;
    Ok(finish("install", green))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(build: bool, check: bool, parts: bool, names: &[&str]) -> Request {
        Request {
            build,
            check,
            parts,
            names: names.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    fn mapped(request: &Request) -> (&'static str, Vec<String>) {
        let plan = plan(request).unwrap();
        (plan.program, plan.args)
    }

    #[test]
    fn maps_arguments_to_scripts() {
        let cases = [
            (
                "everything",
                request(false, false, false, &[]),
                "install/plain.sh",
                vec!["install"],
            ),
            (
                "some parts",
                request(false, false, false, &["sill", "detent"]),
                "install/plain.sh",
                vec!["install", "sill", "detent"],
            ),
            (
                "build",
                request(true, false, false, &[]),
                "install/plain.sh",
                vec!["build"],
            ),
            (
                "build a part",
                request(true, false, false, &["sill"]),
                "install/plain.sh",
                vec!["build", "sill"],
            ),
            (
                "check",
                request(false, true, false, &[]),
                "./install.sh",
                vec!["--check"],
            ),
            (
                "parts",
                request(false, false, true, &[]),
                "install/plain.sh",
                vec!["parts"],
            ),
        ];
        for (name, req, program, args) in cases {
            assert_eq!(
                mapped(&req),
                (program, args.iter().map(|s| (*s).to_owned()).collect()),
                "{name}"
            );
        }
    }

    #[test]
    fn rejects_flags_that_do_not_go_together() {
        let bad = [
            ("two flags", request(true, true, false, &[])),
            ("check with a part", request(false, true, false, &["sill"])),
            ("parts with a part", request(false, false, true, &["sill"])),
        ];
        for (name, req) in bad {
            assert!(plan(&req).is_err(), "{name}");
        }
    }
}
