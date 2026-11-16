//! account-secrets' one error.

use std::sync::Arc;

/// Why the store could not answer. Two errors are equal when they are the same kind of failure.
#[derive(Debug, Clone, thiserror::Error)]
pub enum SecretsError {
    /// Nothing is filed under that key.
    #[error("no such secret")]
    Missing,
    /// The store is locked and the user declined to unlock it.
    #[error("The keychain is locked and was not unlocked.")]
    Locked,
    /// No store is reachable (a headless session without a Secret Service). The text is what a
    /// person may read, so it names what to install, not the protocol.
    #[error(
        "No keychain is running. Install GNOME Keyring (the gnome-keyring package) or KDE Wallet, then sign out and back in."
    )]
    Unavailable,
    /// An item exists but does not decode as a credential.
    #[error("secret item unreadable")]
    Unreadable(#[source] Option<Arc<dyn std::error::Error + Send + Sync>>),
    /// The thread that talks to the store could not be started.
    #[error("The keychain could not be reached.")]
    Worker(#[source] Arc<std::io::Error>),
}

impl SecretsError {
    /// An item that does not decode as a credential, because of `cause`.
    pub fn unreadable(cause: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Unreadable(Some(Arc::new(cause)))
    }
}

impl PartialEq for SecretsError {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Eq for SecretsError {}
