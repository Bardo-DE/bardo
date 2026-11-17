//! What a transport carries, one trait per domain. A transport implements the domains it can
//! serve; a method a transport cannot serve is not on its type, because [`crate::AccountClient`] hands out
//! a domain client only over a transport that implements the domain's trait.

use crate::accounts::{
    AccountChange, AccountRef, AccountView, AccountsError, EditError, NeedsReauth,
};
use crate::authenticated::AuthenticatedStream;
use crate::changes::Changes;
use crate::error::LinkError;
use crate::found::ConsentOffer;
use crate::grants::{GrantsError, ListedAccount};
use account_capability::CapabilityKind;
use account_core::consent::{Availability, Grant, Usage};
use account_core::{
    AccountId, Candidate, DataClass, EndpointUrl, GrantId, ProviderHint, ProviderId,
};
use account_match::Need;
use account_sheet::{EditServers, EditedServers};
use account_wire::ParentWindow;

/// The accounts domain: signing an account in through the sheet.
pub trait AccountsLink: Send + Sync + 'static {
    /// Opens the add-account sheet; the account added.
    fn add(
        &self,
        hint: &ProviderHint,
        window: &ParentWindow,
    ) -> impl Future<Output = Result<AccountId, AccountsError>> + Send;

    /// Opens the sheet that signs a granted account in again.
    fn reauthenticate(
        &self,
        account: &AccountId,
        window: &ParentWindow,
    ) -> impl Future<Output = Result<(), AccountsError>> + Send;
}

/// Hosted only: editing a password account's servers through the sheet. The service checks them
/// before it hands them back; no bus carries it, so it is not on a bus transport's type.
pub trait ServerEdits: AccountsLink {
    /// Opens the server form of `edit`'s account, prefilled, and tries what the person types.
    fn edit_servers(
        &self,
        edit: EditServers,
        window: &ParentWindow,
    ) -> impl Future<Output = Result<EditedServers, EditError>> + Send;
}

/// The grants domain: who may use which account, and the relays a grant opens.
pub trait GrantsLink: Send + Sync + 'static {
    /// The accounts that fit `need` for this app and that it holds a grant for.
    fn candidates(
        &self,
        need: &Need,
        class: DataClass,
        usage: Usage,
    ) -> impl Future<Output = Result<Vec<Candidate>, GrantsError>> + Send;

    /// Whether `need` can be met for this app, revealing no identity.
    fn availability(
        &self,
        need: &Need,
        class: DataClass,
        usage: Usage,
    ) -> impl Future<Output = Result<Availability, GrantsError>> + Send;

    /// The grants this app holds.
    fn grants(&self) -> impl Future<Output = Result<Vec<Grant>, GrantsError>> + Send;

    /// Shows the chooser and consent sheet for `offer` (for the one account it names, when it
    /// names one, without a chooser); the account the person picked, granted. An offer that
    /// names an account is answered with a grant for it or a refusal, never another account.
    fn request_grant(
        &self,
        offer: &ConsentOffer,
        window: &ParentWindow,
    ) -> impl Future<Output = Result<Candidate, GrantsError>> + Send;

    /// Withdraws one of this app's own grants.
    fn revoke(&self, grant: &GrantId) -> impl Future<Output = Result<(), GrantsError>> + Send;

    /// The accounts the person has, for an accounts daemon that keeps a switch per account.
    fn accounts(&self) -> impl Future<Output = Result<Vec<ListedAccount>, GrantsError>> + Send;

    /// Asks the person for the access the account's provider asks only on use, for `kinds`.
    fn extend_access(
        &self,
        account: &AccountId,
        kinds: &[CapabilityKind],
        window: &ParentWindow,
    ) -> impl Future<Output = Result<(), GrantsError>> + Send;

    /// Whether `provider` asks some access of `kind` only on use.
    fn asks_on_use(
        &self,
        provider: &ProviderId,
        kind: CapabilityKind,
    ) -> impl Future<Output = Result<bool, GrantsError>> + Send;

    /// A byte stream to `endpoint`, one of the account's servers, authenticated by a relay.
    fn open_authenticated(
        &self,
        grant: &GrantId,
        endpoint: &EndpointUrl,
    ) -> impl Future<Output = Result<AuthenticatedStream, GrantsError>> + Send;

    /// A byte stream to `origin`, a host the account's provider file declares for the grant's kind
    /// as one its pre-authenticated links point at; the relay adds no credential.
    fn open_linked(
        &self,
        grant: &GrantId,
        origin: &EndpointUrl,
    ) -> impl Future<Output = Result<AuthenticatedStream, GrantsError>> + Send;
}

/// What the shell reads of accountd's accounts, beyond signing one in. The bus only.
pub trait AccountWatch: AccountsLink {
    /// The accounts that need signing in again now, with their labels. Asking is also how accountd
    /// learns this connection, which it then sends every account's `State` changes.
    fn needing_reauth(&self) -> impl Future<Output = Result<Vec<NeedsReauth>, LinkError>> + Send;

    /// One account as accountd shows it to the shell.
    fn view(
        &self,
        account: &AccountRef,
    ) -> impl Future<Output = Result<AccountView, LinkError>> + Send;

    /// [`AccountsLink::reauthenticate`] for an account the shell knows by its object only.
    fn reauthenticate_shown(
        &self,
        account: &AccountRef,
        window: &ParentWindow,
    ) -> impl Future<Output = Result<(), AccountsError>> + Send;

    /// The accounts added and removed, and every change of an account's state, from now on.
    fn watch(&self) -> impl Future<Output = Result<Changes<AccountChange>, LinkError>> + Send;
}
