//! Credentials for a request, kept out of logs.
//!
//! Nothing here serializes: a key is read from where it is kept (a key file, the keyring) into a
//! [`SecretText`], never written back out as part of an endpoint, a config echo or a cassette.

use base_secret::SecretText;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HeaderName(pub String);

/// The header that authenticates a request, if any. Local engines use `None`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum AuthHeader {
    None,
    /// `Authorization: Bearer <secret>`. Refused (`HttpError::Insecure`) on a plain `Tcp` target:
    /// a key goes over TLS or a Unix socket.
    Bearer(SecretText),
    /// `Authorization: Bearer <secret>` to a `Tcp` target on this machine (`localhost`, `127.0.0.0/8`,
    /// `::1`): the explicit opt-in to send a key over plain http on the loopback interface. Any
    /// other plain `Tcp` host is still refused.
    LoopbackBearer(SecretText),
    /// A vendor's own header (`x-api-key`).
    Header {
        name: HeaderName,
        value: SecretText,
    },
}

/// A header sent on every request beside the authenticating one.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ExtraHeader {
    pub name: HeaderName,
    pub value: SecretText,
}
