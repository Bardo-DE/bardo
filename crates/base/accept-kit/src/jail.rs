//! The argument list of `packaging/harness/jail.sh`, with the script's own rules.
//!
//! A pure port of its option parsing, so a plan that would make the script print its usage fails here, with the
//! reason. xtask still runs the script; this builds (and checks) what it is given.

use std::fmt;
use std::path::PathBuf;

/// The script's exit code for "the jail cannot run here" (no bubblewrap, no namespace, no
/// `/dev/udmabuf`, a missing `--bind` directory): a run reports FAIL with the reason, never a
/// pass.
pub const UNAVAILABLE: i32 = 3;

/// The most soft GPU nodes the script fabricates (render minors 128..191).
pub const MAX_GPUS: u32 = 64;

/// What one jail run is given.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    /// Directories made writable inside (`--bind`), in order.
    pub binds: Vec<PathBuf>,
    /// The scratch directory instead of a fresh temporary one (`--scratch`).
    pub scratch: Option<PathBuf>,
    /// More variables set inside (`--env K=V`); the environment is otherwise cleared.
    pub env: Vec<(String, String)>,
    /// How many soft GPU nodes to fabricate (`--fabricate-gpus`); `0` passes no flag.
    pub fabricate_gpus: u32,
}

/// Why a [`Plan`] cannot be given to the jail.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlanError {
    /// More GPUs than the script can fabricate.
    TooManyGpus(u32),
    /// An `--env` name that is empty or contains `=`.
    BadEnvName(String),
    /// A path that is not valid UTF-8 (the script's arguments are text).
    NotUtf8(PathBuf),
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyGpus(n) => {
                write!(f, "{n} fabricated GPUs; the jail makes at most {MAX_GPUS}")
            }
            Self::BadEnvName(name) => {
                write!(f, "{name:?} is not a variable name (empty, or has `=`)")
            }
            Self::NotUtf8(path) => write!(f, "{} is not valid UTF-8", path.display()),
        }
    }
}

impl std::error::Error for PlanError {}

impl Plan {
    /// The options for `jail.sh`, without the `--` and the command.
    ///
    /// # Errors
    /// The first rule the script would refuse the plan by.
    pub fn argv(&self) -> Result<Vec<String>, PlanError> {
        if self.fabricate_gpus > MAX_GPUS {
            return Err(PlanError::TooManyGpus(self.fabricate_gpus));
        }
        let text = |path: &PathBuf| {
            path.to_str()
                .map(str::to_owned)
                .ok_or_else(|| PlanError::NotUtf8(path.clone()))
        };
        let mut argv = Vec::new();
        for dir in &self.binds {
            argv.extend(["--bind".to_owned(), text(dir)?]);
        }
        if let Some(scratch) = &self.scratch {
            argv.extend(["--scratch".to_owned(), text(scratch)?]);
        }
        if self.fabricate_gpus > 0 {
            argv.extend([
                "--fabricate-gpus".to_owned(),
                self.fabricate_gpus.to_string(),
            ]);
        }
        for (key, value) in &self.env {
            if key.is_empty() || key.contains('=') {
                return Err(PlanError::BadEnvName(key.clone()));
            }
            argv.extend(["--env".to_owned(), format!("{key}={value}")]);
        }
        Ok(argv)
    }

    /// The whole argument list: the options, `--`, then `command`.
    ///
    /// # Errors
    /// As [`Self::argv`].
    pub fn argv_for<I, S>(&self, command: I) -> Result<Vec<String>, PlanError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut argv = self.argv()?;
        argv.push("--".to_owned());
        argv.extend(command.into_iter().map(Into::into));
        Ok(argv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_plan_passes_no_option() {
        assert_eq!(Plan::default().argv().unwrap(), Vec::<String>::new());
    }

    #[test]
    fn the_options_come_in_the_scripts_words_and_order() {
        let plan = Plan {
            binds: vec!["/w".into(), "/t".into()],
            scratch: Some("/s".into()),
            env: vec![("A".into(), "1".into()), ("B".into(), "x=y".into())],
            fabricate_gpus: 2,
        };
        assert_eq!(
            plan.argv_for(["cargo", "nextest"]).unwrap(),
            [
                "--bind",
                "/w",
                "--bind",
                "/t",
                "--scratch",
                "/s",
                "--fabricate-gpus",
                "2",
                "--env",
                "A=1",
                "--env",
                "B=x=y",
                "--",
                "cargo",
                "nextest"
            ]
        );
    }

    #[test]
    fn what_the_script_would_refuse_is_refused() {
        let gpus = Plan {
            fabricate_gpus: 65,
            ..Plan::default()
        };
        assert_eq!(gpus.argv(), Err(PlanError::TooManyGpus(65)));
        let _most = Plan {
            fabricate_gpus: 64,
            ..Plan::default()
        }
        .argv()
        .expect("64 is the most the script fabricates");
        for name in ["", "A=B"] {
            let plan = Plan {
                env: vec![(name.into(), "v".into())],
                ..Plan::default()
            };
            assert_eq!(plan.argv(), Err(PlanError::BadEnvName(name.into())));
        }
    }
}
