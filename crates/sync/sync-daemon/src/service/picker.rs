//! `org.bardo.Photos1.Picker` on syncd's object (`sync_bus::PhotosPickerProxy`): the Google
//! Photos Picker for the Photos app. Served in a build with the `photos-picker` feature; an
//! account answers through it while its Photos row in Settings is on. The caller is identified as `Sync1`'s are (the connection's, by
//! account-dbus's `Callers`), and only an app that owns the Photos datasets (`org.bardo.Photos`)
//! may call it. Settings and the porter daemons see datasets but are not the app that imports
//! photos, so they are `Denied`, as they are for `Sync1.Resolve`.
//!
//! - An account with no Picker (unknown, not Google, the person did not grant the picker scope,
//!   Photos not granted to syncd) answers `NoFittingAccount`.
//! - A malformed account or session word is `InvalidArgs`.
//! - `Import` before the person is done is `org.bardo.Photos1.Error.NotYet`; a session Google
//!   does not have is `org.bardo.Photos1.Error.NoSuchSession`; the Picker not answering is
//!   `Unavailable`; anything else `Failed`.
//! - Nothing but ids, the page to open, a state word and the files' paths crosses the bus.

use super::errors::RefusedError;
use account_callers::Callers;
use account_wire::Refusal;
use dbus_kit::Call;
use std::sync::Arc;
use sync_bus::SYNC_PATH;
use sync_bus::{PICKER_PICKED, PICKER_WAITING, PhotosPickerServed, PhotosPickerServer};
use sync_driver::hub::Access;
use sync_photos::desk::{PickerDesk, Started};
use sync_photos::google::{PickerError, PickerState, SessionId};
use sync_xdg::AccountDir;
use zbus::Connection;
use zbus::message::Header;

struct Core<C, D> {
    callers: C,
    desk: D,
    owners: Access,
}

#[cfg(feature = "photos-picker")]
impl<C: Callers, D: PickerDesk> Core<C, D> {
    /// Checks the caller is the Photos app, and the words.
    async fn admit(
        &self,
        header: &Header<'_>,
        account: &str,
        session: Option<&str>,
    ) -> Result<(AccountDir, Option<SessionId>), RefusedError> {
        let sender = header
            .sender()
            .ok_or_else(|| RefusedError::access_denied("no sender"))?;
        let caller = self
            .callers
            .caller_of(sender.as_str())
            .await
            .ok_or_else(|| RefusedError::access_denied("syncd does not know this caller"))?;
        if !self.owners.owned_by(&caller) {
            return Err(RefusedError::of(Refusal::Denied));
        }
        let account = AccountDir::parse(account)
            .ok_or_else(|| RefusedError::invalid("not an account directory name"))?;
        let session = session
            .map(|text| {
                SessionId::parse(text)
                    .ok_or_else(|| RefusedError::invalid("the session id is not a plain token"))
            })
            .transpose()?;
        Ok((account, session))
    }
}

#[cfg(feature = "photos-picker")]
fn refused(error: PickerError) -> RefusedError {
    match error {
        PickerError::NotYet => RefusedError::picker_not_yet(),
        PickerError::Refused(404) => RefusedError::picker_no_such_session(),
        PickerError::Unreached(_) => RefusedError::of(Refusal::Unavailable),
        PickerError::BadSession => RefusedError::invalid("the session id is not a plain token"),
        other => RefusedError::failed(with_cause(&other)),
    }
}

/// The text of `error` and of what caused it, for a wire error that holds only text.
#[cfg(feature = "photos-picker")]
fn with_cause(error: &PickerError) -> String {
    match std::error::Error::source(error) {
        Some(cause) => format!("{error}: {cause}"),
        None => error.to_string(),
    }
}

#[cfg(feature = "photos-picker")]
fn none_here() -> RefusedError {
    RefusedError::of(Refusal::NoFittingAccount)
}

struct PickerObject<C, D>(Arc<Core<C, D>>);

#[cfg(feature = "photos-picker")]
impl<C: Callers, D: PickerDesk> PhotosPickerServer for PickerObject<C, D> {
    type Error = RefusedError;

    async fn start(
        &self,
        Call { header, .. }: Call<'_>,
        account: String,
    ) -> Result<(String, String, u32), RefusedError> {
        let (account, _) = self.0.admit(&header, &account, None).await?;
        let started = self
            .0
            .desk
            .start(&account)
            .await
            .ok_or_else(none_here)?
            .map_err(refused)?;
        Ok((started.session, started.picker_uri, started.poll_s))
    }

    async fn poll(
        &self,
        Call { header, .. }: Call<'_>,
        account: String,
        session: String,
    ) -> Result<String, RefusedError> {
        let (account, session) = self.0.admit(&header, &account, Some(&session)).await?;
        let session = session.ok_or_else(none_here)?;
        let state = self
            .0
            .desk
            .poll(&account, &session)
            .await
            .ok_or_else(none_here)?
            .map_err(refused)?;
        Ok(match state {
            PickerState::Waiting => PICKER_WAITING,
            PickerState::Picked => PICKER_PICKED,
        }
        .to_owned())
    }

    async fn import(
        &self,
        Call { header, .. }: Call<'_>,
        account: String,
        session: String,
    ) -> Result<Vec<String>, RefusedError> {
        let (account, session) = self.0.admit(&header, &account, Some(&session)).await?;
        let session = session.ok_or_else(none_here)?;
        self.0
            .desk
            .import(&account, &session)
            .await
            .ok_or_else(none_here)?
            .map_err(refused)
    }

    async fn cancel(
        &self,
        Call { header, .. }: Call<'_>,
        account: String,
        session: String,
    ) -> Result<(), RefusedError> {
        let (account, session) = self.0.admit(&header, &account, Some(&session)).await?;
        let session = session.ok_or_else(none_here)?;
        if self.0.desk.cancel(&account, &session).await {
            Ok(())
        } else {
            Err(none_here())
        }
    }
}

/// Serves `org.bardo.Photos1.Picker` beside `Sync1` on `connection`: `desk` has the Pickers,
/// `owners` is who may call (the Photos app), `callers` says who the senders are.
pub async fn serve_picker<C: Callers, D: PickerDesk>(
    connection: &Connection,
    desk: D,
    owners: Access,
    callers: C,
) -> zbus::Result<()> {
    let core = Arc::new(Core {
        callers,
        desk,
        owners,
    });
    connection
        .object_server()
        .at(SYNC_PATH, PhotosPickerServed(PickerObject(core)))
        .await?;
    Ok(())
}
