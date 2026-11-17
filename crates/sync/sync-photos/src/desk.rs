//! The Google Photos Picker of each account syncd runs Photos for: the registry the storage
//! supervisor fills and the bus object reads ([`Pickers`]), and the seam the bus object asks
//! through ([`PickerDesk`]), so the object never holds a [`GooglePicker`] of its own.

use crate::google::{GooglePicker, PickerError, PickerState, SessionId};
use account_client::GrantsLink;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};
use sync_xdg::AccountDir;

/// The Pickers of the accounts syncd runs Google Photos for, shared between the supervisor
/// that starts them and the bus object that serves them.
pub struct Pickers<T: GrantsLink>(Arc<Mutex<BTreeMap<AccountDir, Arc<GooglePicker<T>>>>>);

impl<T: GrantsLink> Clone for Pickers<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T: GrantsLink> Default for Pickers<T> {
    fn default() -> Self {
        Self(Arc::default())
    }
}

impl<T: GrantsLink> std::fmt::Debug for Pickers<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.accounts()).finish()
    }
}

impl<T: GrantsLink> Pickers<T> {
    /// Replaces the set with `now`.
    pub fn set(&self, now: BTreeMap<AccountDir, Arc<GooglePicker<T>>>) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = now;
    }

    fn accounts(&self) -> Vec<AccountDir> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .keys()
            .cloned()
            .collect()
    }

    fn of(&self, account: &AccountDir) -> Option<Arc<GooglePicker<T>>> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(account)
            .cloned()
    }
}

/// What a session looks like to the bus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Started {
    /// The session id.
    pub session: String,
    /// The page the app opens.
    pub picker_uri: String,
    /// Seconds between polls.
    pub poll_s: u32,
}

/// The Picker of an account, as the bus object asks for it.
pub trait PickerDesk: Send + Sync + 'static {
    /// Starts a session; `None` when the account has no Picker.
    fn start(
        &self,
        account: &AccountDir,
    ) -> impl Future<Output = Option<Result<Started, PickerError>>> + Send;
    /// The session's state; `None` when the account has no Picker.
    fn poll(
        &self,
        account: &AccountDir,
        session: &SessionId,
    ) -> impl Future<Output = Option<Result<PickerState, PickerError>>> + Send;
    /// Imports what was picked: the files' paths; `None` when the account has no Picker.
    fn import(
        &self,
        account: &AccountDir,
        session: &SessionId,
    ) -> impl Future<Output = Option<Result<Vec<String>, PickerError>>> + Send;
    /// Ends the session; whether the account has a Picker.
    fn cancel(
        &self,
        account: &AccountDir,
        session: &SessionId,
    ) -> impl Future<Output = bool> + Send;
}

impl<T: GrantsLink + 'static> PickerDesk for Pickers<T> {
    async fn start(&self, account: &AccountDir) -> Option<Result<Started, PickerError>> {
        let picker = self.of(account)?;
        Some(picker.start().await.map(|s| Started {
            session: s.id.to_string(),
            picker_uri: s.picker_uri,
            poll_s: u32::try_from(s.poll_every.as_secs().max(1)).unwrap_or(u32::MAX),
        }))
    }

    async fn poll(
        &self,
        account: &AccountDir,
        session: &SessionId,
    ) -> Option<Result<PickerState, PickerError>> {
        Some(self.of(account)?.poll(session).await)
    }

    async fn import(
        &self,
        account: &AccountDir,
        session: &SessionId,
    ) -> Option<Result<Vec<String>, PickerError>> {
        let picker = self.of(account)?;
        Some(picker.import(session).await.map(|imported| {
            imported
                .files
                .iter()
                .map(|f| f.to_string_lossy().into_owned())
                .collect()
        }))
    }

    async fn cancel(&self, account: &AccountDir, session: &SessionId) -> bool {
        match self.of(account) {
            Some(picker) => {
                picker.cancel(session).await;
                true
            }
            None => false,
        }
    }
}
