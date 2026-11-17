//! What lending this computer's models to the person's other computers touches: the accounts service's file for
//! it and the person's own service manager. Behind one handle that the environment carries, so a
//! test hands in scratch folders and a manager that only writes down what it was asked.

use inference_client::{
    LendingConfig, LendingError, LendingState, SessionUnits, TailnetLending, UnitError, UnitManager,
};
use settings_app_files::ConfigHome;
use settings_app_link::lending::{Lend, Lender};
use settings_app_model::{Existing, LendFailure, Work};
use std::fmt::Debug;
use std::path::Path;
use std::sync::Arc;

/// What the log says when the computer's own services cannot be reached.
const NO_SERVICES: &str = "the session's service manager could not be reached";

fn failure_of(error: &LendingError) -> LendFailure {
    match error {
        LendingError::ShippedFileMissing(_) => LendFailure::NotInstalled,
        LendingError::CouldNotWrite(_) => LendFailure::CouldNotSave,
        LendingError::ReloadFailed(_) => LendFailure::CouldNotReload,
        LendingError::RestartFailed(_) => LendFailure::CouldNotRestart,
    }
}

fn existing_of(state: LendingState) -> Existing {
    match state {
        LendingState::NotInstalled => Existing::Absent,
        LendingState::Installed => Existing::Same,
        LendingState::Differs => Existing::Different,
    }
}

/// Lending by `config` over a manager that is given.
#[derive(Debug)]
struct Over<M>(TailnetLending<M>);

impl<M: UnitManager + Debug + 'static> Lend for Over<M> {
    fn existing(&self) -> Existing {
        existing_of(self.0.state())
    }

    fn enable(&self) -> Work<'_, Result<(), LendFailure>> {
        Box::pin(async move { self.0.enable().await.map_err(|error| failure_of(&error)) })
    }

    fn disable(&self) -> Work<'_, Result<(), LendFailure>> {
        Box::pin(async move { self.0.disable().await.map_err(|error| failure_of(&error)) })
    }
}

/// A manager that is never asked: reading what is there needs none.
#[derive(Debug)]
struct NoUnits;

impl UnitManager for NoUnits {
    #[expect(
        clippy::unused_async_trait_impl,
        reason = "UnitManager is async; this manager is never asked and answers at once"
    )]
    async fn reload(&self) -> Result<(), UnitError> {
        Err(UnitError(NO_SERVICES.to_owned()))
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "UnitManager is async; this manager is never asked and answers at once"
    )]
    async fn restart(&self, _unit: &'static str) -> Result<(), UnitError> {
        Err(UnitError(NO_SERVICES.to_owned()))
    }
}

/// The real thing: the person's session, reached when a change is made.
#[derive(Debug)]
struct Session(LendingConfig);

impl Session {
    async fn lending(&self) -> Result<TailnetLending<SessionUnits>, LendFailure> {
        let units = SessionUnits::session().await.map_err(|why| {
            eprintln!("bardo-settings: sharing: the session's services are out of reach: {why:?}");
            LendFailure::NoServices
        })?;
        Ok(TailnetLending::new(self.0.clone(), units))
    }
}

impl Lend for Session {
    fn existing(&self) -> Existing {
        existing_of(TailnetLending::new(self.0.clone(), NoUnits).state())
    }

    fn enable(&self) -> Work<'_, Result<(), LendFailure>> {
        Box::pin(async move {
            let lending = self.lending().await?;
            lending.enable().await.map_err(|error| failure_of(&error))
        })
    }

    fn disable(&self) -> Work<'_, Result<(), LendFailure>> {
        Box::pin(async move {
            let lending = self.lending().await?;
            lending.disable().await.map_err(|error| failure_of(&error))
        })
    }
}

/// This computer has no home folder: nothing can be kept.
#[derive(Debug)]
struct Nowhere;

impl Lend for Nowhere {
    fn existing(&self) -> Existing {
        Existing::Absent
    }

    fn enable(&self) -> Work<'_, Result<(), LendFailure>> {
        Box::pin(async { Err(LendFailure::NoHome) })
    }

    fn disable(&self) -> Work<'_, Result<(), LendFailure>> {
        Box::pin(async { Err(LendFailure::NoHome) })
    }
}

/// The person's own session: the shipped file is looked for where the environment says
/// (`XDG_DATA_HOME`, `XDG_DATA_DIRS`), the setting kept under `XDG_CONFIG_HOME`.
pub fn from_env() -> Lender {
    LendingConfig::from_lookup(|key| std::env::var_os(key))
        .map_or_else(|| Lender::new(Arc::new(Nowhere)), session)
}

/// The person's own session, with the accounts service's file at `<prefix>/share/accounts/` and the setting
/// kept under `config_home`.
#[must_use]
pub fn under_prefix(prefix: &Path, config_home: &ConfigHome) -> Lender {
    session(LendingConfig::under_prefix(
        prefix,
        config_home.0.join("systemd/user"),
    ))
}

/// The person's own session over `config`.
#[must_use]
pub fn session(config: LendingConfig) -> Lender {
    Lender::new(Arc::new(Session(config)))
}

/// Lending by `config` over `manager`: what a test hands in.
pub fn over<M: UnitManager + Debug + 'static>(config: LendingConfig, manager: M) -> Lender {
    Lender::new(Arc::new(Over(TailnetLending::new(config, manager))))
}
