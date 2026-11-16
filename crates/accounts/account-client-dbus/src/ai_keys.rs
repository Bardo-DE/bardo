//! The person's AI keys, as an app asks for them (`org.bardo.Accounts1.Keys` on accountd). An AI
//! key is accountd's, and inferd is the one that sends it to an AI company: an app never sees
//! one. What an app gets is the person's yes to use their keys, a permit ([`GrantId`]), after
//! accountd shows the consent sheet; an app that holds a permit already is answered with it and
//! no sheet is drawn.
//!
//! ```ignore
//! let keys = bus.ai_keys();
//! match keys.request_key(DataClass::Documents, Usage::Interactive, &window).await {
//!     Ok(permit) => { /* ask inferd for a session as usual; the permit is what lets it */ }
//!     Err(AiKeysError::Refused(Refusal::Dismissed)) => { /* the person closed the sheet */ }
//!     Err(AiKeysError::Refused(Refusal::NoFittingAccount)) => {
//!         // no key is added: send the person to add one (`Accounts::add`), then ask again
//!     }
//!     Err(other) => report(other),
//! }
//! ```
//!
//! Adding a key is `Accounts::add`, the same sheet as for any account. Settings lists and revokes
//! permits through accountd's settings rows, not here. The agent, the agent launcher, the
//! computer-use daemon, the terminal and the compositor are refused.

use crate::Bus;
use crate::failure::{ai_keys_error, link_error};
use account_client::{AiKeysError, LinkError, Malformed};
use account_core::consent::Usage;
use account_core::{DataClass, GrantId};
use account_dbus::accounts::sheet::{SheetKind, reply_of};
use account_dbus::accounts::{Abandon, KeysProxy, ServiceRoot, Sheet, SheetError};
use account_wire::{AccountsReply, ParentWindow};
use base_ids::word::word as word_of;

fn malformed(what: Malformed) -> AiKeysError {
    AiKeysError::Link(LinkError::Malformed(what))
}

/// The `parent_window` argument: no parent is the empty string.
fn wire(window: &ParentWindow) -> String {
    match window {
        ParentWindow::Handle(handle) => handle.clone(),
        ParentWindow::Unparented => String::new(),
    }
}

fn sheet_error(error: SheetError) -> AiKeysError {
    match error {
        SheetError::Bus(error) => ai_keys_error(&error),
        SheetError::Gone => AiKeysError::Link(LinkError::Unreachable),
        SheetError::Unreadable => malformed(Malformed::Sheet),
    }
}

/// The Keys interface of accountd, over the bus.
#[derive(Debug, Clone)]
pub struct AiKeys {
    bus: Bus,
}

impl Bus {
    /// The handle on ai keys.
    #[must_use]
    pub fn ai_keys(&self) -> AiKeys {
        AiKeys { bus: self.clone() }
    }
}

impl AiKeys {
    /// Shows the person accountd's consent sheet for this app's use of one of their AI keys, for
    /// data of `class` used as `usage` says, over `window`. The permit it answers is the person's
    /// yes, within the scope they chose.
    ///
    /// # Errors
    ///
    /// [`AiKeysError`] when the sheet cannot be shown, the person refuses it, or the reply is
    /// malformed.
    pub async fn request_key(
        &self,
        class: DataClass,
        usage: Usage,
        window: &ParentWindow,
    ) -> Result<GrantId, AiKeysError> {
        let proxy: KeysProxy<'static> = self
            .bus
            .accounts
            .proxy(&self.bus.connection)
            .await
            .map_err(|e| AiKeysError::Link(link_error(&e)))?;
        let (class, usage, window) = (word_of(&class), word_of(&usage), wire(window));
        let mut listener = Sheet::subscribe_on(&self.bus.connection, ServiceRoot::Accounts)
            .await
            .map_err(|e| ai_keys_error(&e))?;
        // Armed before the call: a dropped call may already have made the sheet.
        let mut abandon = Abandon(listener.closer(None));
        let path = match proxy
            .request_key(class, usage, &window, &listener.options())
            .await
        {
            Ok(path) => path,
            Err(error) => {
                abandon.answered();
                return Err(ai_keys_error(&error));
            }
        };
        abandon.0 = listener.closer(Some(path.clone()));
        let (code, results) = listener.response(&path).await.map_err(sheet_error)?;
        abandon.answered();
        match reply_of(SheetKind::Choose, code, results)
            .ok()
            .ok_or(malformed(Malformed::Reply))?
        {
            // The answer is the chosen key; its `grant` is the permit's id.
            AccountsReply::Chosen(candidate) => Ok(candidate.grant),
            AccountsReply::Refused(refusal) => Err(AiKeysError::Refused(refusal)),
            _ => Err(malformed(Malformed::Reply)),
        }
    }
}
