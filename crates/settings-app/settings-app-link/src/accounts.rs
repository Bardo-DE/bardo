//! The account service as the Accounts pane meets it.
//!
//! That is the accounts the live modules list, and the sheet that adds an account (an identity or
//! an AI key, through accountd), asked of the owner of the service's name that was confirmed.

use settings_app_model::{Absence, AccountsPane, AddEnd, Work};
use std::fmt::Debug;

/// What the Accounts pane asks of the account service.
pub trait AccountsLink: Debug + Send + Sync {
    /// The accounts the modules list now, read once; never starts a daemon.
    fn read(&self) -> Work<'_, Result<AccountsPane, Absence>>;
    /// Asks accountd for the add sheet and waits for it to end.
    fn add_account(&self) -> Work<'_, AddEnd>;
}
