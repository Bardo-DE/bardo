//! Why the `computer-use-eval` program stops: a flag it cannot use, or a step of the run that failed. The
//! program prints the error and exits with status 2.

use computer_use_eval::flags::FlagError;
use computer_use_eval::live::LiveError;
use computer_use_eval::run::SetupError;
use std::path::PathBuf;

/// A command-line flag that cannot be used.
#[derive(Debug, thiserror::Error)]
pub enum ArgError {
    /// A flag that cannot be used.
    #[error(transparent)]
    Flag(#[from] FlagError),
    /// A platform name the chord keymap does not know.
    #[error("{0}")]
    Platform(#[from] keymap::PlatformError),
}

/// Why a run could not be played or written.
#[derive(Debug, thiserror::Error)]
pub enum PlayError {
    /// The engine needs a flag that was not given.
    #[error("{0}")]
    Missing(&'static str),
    /// A task could not be set up or played.
    #[error(transparent)]
    Setup(#[from] SetupError),
    /// The catalogue entry cannot drive this eval.
    #[error(transparent)]
    Plan(#[from] LiveError),
    /// A file could not be read or written.
    #[error("{}: {source}", path.display())]
    Io {
        /// The file or directory.
        path: PathBuf,
        /// What went wrong.
        source: std::io::Error,
    },
    /// The cassette file is not a cassette.
    #[error("cassette: {0}")]
    CassetteRead(#[from] toml::de::Error),
    /// The cassette could not be written as TOML.
    #[error("cassette: {0}")]
    CassetteWrite(#[from] toml::ser::Error),
}

impl PlayError {
    /// A file error for `path`, to hand to `map_err`.
    pub fn io(path: &std::path::Path) -> impl FnOnce(std::io::Error) -> PlayError {
        let path = path.to_owned();
        move |source| PlayError::Io { path, source }
    }
}
