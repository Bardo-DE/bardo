//! The package manager, as a seam: node packages are installed by `npm`, which checks them.

use std::path::Path;
use std::process::Command;

/// Why a program did not run to success.
#[derive(Debug, Clone, thiserror::Error)]
pub enum RunError {
    /// The program could not be started.
    #[error("{program} could not be started")]
    Start {
        /// The program.
        program: String,
        /// Why it did not start.
        #[source]
        source: std::sync::Arc<std::io::Error>,
    },
    /// The program ran and did not succeed.
    #[error("{0} did not finish")]
    Failed(String),
}

/// Runs a program to its end.
pub trait Run {
    /// Runs `program` with `args`, working in `dir`.
    ///
    /// # Errors
    ///
    /// Returns [`RunError`] when the program did not run to success.
    fn run(&self, program: &str, args: &[String], dir: &Path) -> Result<(), RunError>;
}

/// Runs the real program.
#[derive(Debug, Clone, Copy)]
pub struct System;

impl Run for System {
    fn run(&self, program: &str, args: &[String], dir: &Path) -> Result<(), RunError> {
        let status = Command::new(program)
            .args(args)
            .current_dir(dir)
            .status()
            .map_err(|source| RunError::Start {
                program: program.to_owned(),
                source: std::sync::Arc::new(source),
            })?;
        status
            .success()
            .then_some(())
            .ok_or_else(|| RunError::Failed(program.to_owned()))
    }
}
