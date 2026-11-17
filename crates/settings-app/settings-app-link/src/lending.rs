//! The switch "Let my other computers use this computer's models".
//!
//! What it asks of the computer, behind one handle that the environment carries, so a test hands in scratch folders and a
//! manager that only writes down what it was asked.

use crate::deadline::{Deadline, peer};
use base_time::deadline::within;
use settings_app_model::{Existing, LendFailure, Work};
use std::fmt::Debug;
use std::sync::Arc;

/// The two things the switch does, and what is there before it does them.
pub trait Lend: Debug + Send + Sync {
    /// What is set up now.
    fn existing(&self) -> Existing;
    /// Sets sharing up.
    fn enable(&self) -> Work<'_, Result<(), LendFailure>>;
    /// Takes sharing away.
    fn disable(&self) -> Work<'_, Result<(), LendFailure>>;
}

/// How the switch "Let my other computers use this computer's models" reaches the files and the
/// service manager.
#[derive(Debug, Clone)]
pub struct Lender(Arc<dyn Lend>);

impl Lender {
    /// The lender that does what `lend` does.
    pub fn new(lend: Arc<dyn Lend>) -> Lender {
        Lender(lend)
    }

    /// What is set up now.
    #[must_use]
    pub fn existing(&self) -> Existing {
        self.0.existing()
    }

    /// Sets sharing up; why not when it could not be.
    ///
    /// # Errors
    ///
    /// The `LendFailure` the change ended in, or `NoServices` when the session's services do not
    /// answer in time.
    pub async fn enable(&self) -> Result<(), LendFailure> {
        within(Deadline::ACTION, peer::SERVICES, self.0.enable())
            .await
            .map_err(|_silent| LendFailure::NoServices)?
    }

    /// Takes sharing away; why not when it could not be.
    ///
    /// # Errors
    ///
    /// As [`Self::enable`].
    pub async fn disable(&self) -> Result<(), LendFailure> {
        within(Deadline::ACTION, peer::SERVICES, self.0.disable())
            .await
            .map_err(|_silent| LendFailure::NoServices)?
    }
}
