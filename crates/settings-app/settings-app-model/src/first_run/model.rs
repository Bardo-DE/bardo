//! What the first-run window shows and what the person has chosen so far.

use crate::accounts::{AccountKey, AccountStatus, AccountsPane};

/// A local runtime the window offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Runtime {
    /// Its account.
    pub key: AccountKey,
    /// What it is called ("Ollama").
    pub label: String,
    /// How it stands: a stopped one is listed and cannot be picked.
    pub status: AccountStatus,
}

impl Runtime {
    /// Whether it can be picked now.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.status == AccountStatus::Ok
    }

    /// The runtimes `pane` lists, in its order.
    #[must_use]
    pub fn all_of(pane: &AccountsPane) -> Vec<Runtime> {
        pane.local_runtimes()
            .map(|account| Runtime {
                key: account.key.clone(),
                label: account.label.clone(),
                status: account.status.clone(),
            })
            .collect()
    }
}

/// Where the window is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Waiting for the person.
    Asking,
    /// Writing what was chosen.
    Saving,
    /// Written: the window closes.
    Done,
}

/// The window's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstRun {
    /// Whether AI stays on this computer; on until the person says otherwise.
    pub local_only: bool,
    /// Where it is.
    pub stage: Stage,
}

impl Default for FirstRun {
    fn default() -> Self {
        FirstRun {
            local_only: true,
            stage: Stage::Asking,
        }
    }
}
