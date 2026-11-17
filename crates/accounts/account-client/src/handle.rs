//! `AccountClient`: the app's handle on the account service, over a transport.

use crate::accounts::Accounts;
use crate::grants::Grants;
use crate::link::{AccountsLink, GrantsLink};
use std::sync::Arc;

/// The app's handle on the account service.
///
/// `T` is the transport: the bus (`account-client-dbus`) or the service hosted in
/// process (`account-client-inproc`). A domain accessor exists only over a transport that carries
/// the domain, so a call a transport cannot serve does not compile.
#[derive(Debug)]
pub struct AccountClient<T> {
    link: Arc<T>,
}

impl<T> Clone for AccountClient<T> {
    fn clone(&self) -> Self {
        Self {
            link: Arc::clone(&self.link),
        }
    }
}

impl<T> AccountClient<T> {
    /// Over a transport the app built.
    pub fn over(link: T) -> Self {
        Self {
            link: Arc::new(link),
        }
    }
}

impl<T: AccountsLink> AccountClient<T> {
    /// Signing an account in.
    #[must_use]
    pub fn accounts(&self) -> Accounts<T> {
        Accounts {
            link: Arc::clone(&self.link),
        }
    }
}

impl<T: GrantsLink> AccountClient<T> {
    /// Who may use which account, and the relays a grant opens.
    #[must_use]
    pub fn grants(&self) -> Grants<T> {
        Grants {
            link: Arc::clone(&self.link),
        }
    }
}
