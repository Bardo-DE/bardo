//! The app-facing API of the account service (design/31 §5.1). An app asks for a capability,
//! never a brand. [`Porter`] is the handle, over a transport: [`Porter::accounts`] signs an account
//! in; [`Porter::grants`] finds an account by capability, asks for consent and opens the relays a
//! grant allows. The transports are `account-client-dbus` (accountd on the session bus) and
//! `account-client-inproc` (the service hosted in the app); this crate names neither a bus nor a
//! service.
//!
//! ```no_run
//! use account_client::{ConsentOffer, Found, GrantsError, GrantsLink, Grants};
//! use account_core::consent::Usage;
//! use account_core::{Candidate, DataClass};
//! use account_match::Need;
//! use account_wire::ParentWindow;
//!
//! async fn photos_account<T: GrantsLink>(
//!     grants: &Grants<T>,
//!     need: &Need,
//!     window: &ParentWindow,
//! ) -> Result<Option<Candidate>, GrantsError> {
//!     Ok(match grants.find(need, DataClass::Photos, Usage::Interactive).await? {
//!         Found::One(candidate) => Some(candidate),
//!         Found::Several(list) => list.into_iter().next(), // Bardo's AccountPicker lets the person pick
//!         Found::NeedsConsent(offer) => Some(grants.request_grant(&offer, window).await?),
//!         Found::None(_why) => None, // EmptyState + "Add Account…"
//!     })
//! }
//! ```
//!
//! No feature is on by default, and the crate reaches no zbus and no service crate. No
//! credential crosses a bus: over D-Bus an app asks accountd's `Grants1` for a grant and for a
//! stream to the account's server that accountd has signed in ([`Grants`]), `Identities1` for
//! "Continue as" and for a permit to use the person's AI keys (both in `account-client-dbus`).
//! Inference is `inference-client`'s.

mod accounts;
mod ai_accounts;
mod ai_keys;
mod authenticated;
mod changes;
mod error;
mod found;
mod grants;
mod identities;
mod link;
mod porter;
mod spaces;

pub use accounts::{
    AccountChange, AccountRef, AccountView, Accounts, AccountsError, EditError, NeedsReauth,
};
pub use ai_accounts::AiAccountsError;
pub use ai_keys::AiKeysError;
pub use authenticated::AuthenticatedStream;
pub use changes::Changes;
pub use error::{LinkError, Malformed, MalformedError, Unwritable};
pub use found::{ConsentOffer, Found, NoAccount, found};
pub use grants::{Grants, GrantsError, ListedAccount};
pub use identities::{IdentitiesError, Picked};
pub use link::{AccountWatch, AccountsLink, GrantsLink, ServerEdits};
pub use porter::Porter;
pub use spaces::SpacesError;
