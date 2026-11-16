//! Secret text that cannot leak by accident: a redacting `Debug`, bytes wiped on drop, constant-time
//! equality, and no `Serialize` (a field that must write one names [`write_secret`]).

use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use std::fmt;
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Secret text, held in a [`secrecy::SecretString`].
///
/// Its `Debug` never shows it, so it cannot
/// leak into a log, a panic or an error; its bytes are overwritten when it is dropped, so a freed
/// buffer does not keep it; two of them compare in constant time (`subtle`). It has no
/// `Serialize`: a value that must be written goes through [`write_secret`] or
/// [`write_secret_option`], named at the field that writes it. (A copy made by growing a `String`
/// before it was wrapped, or by a caller of [`SecretText::expose`], is that code's to wipe.)
#[derive(Clone)]
pub struct SecretText(SecretString);

impl SecretText {
    /// Wraps a secret.
    pub fn new(text: impl Into<String>) -> Self {
        Self(SecretString::from(text.into()))
    }

    /// The secret itself, for the one call that must present it.
    #[must_use]
    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }
}

impl PartialEq for SecretText {
    fn eq(&self, other: &Self) -> bool {
        self.expose()
            .as_bytes()
            .ct_eq(other.expose().as_bytes())
            .into()
    }
}

impl Eq for SecretText {}

impl Zeroize for SecretText {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl ZeroizeOnDrop for SecretText {}

impl fmt::Debug for SecretText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

impl<'de> Deserialize<'de> for SecretText {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        SecretString::deserialize(deserializer).map(Self)
    }
}

/// Writes a secret as its text.
///
/// The one way a [`SecretText`] is serialised, named by
/// `#[serde(serialize_with = "...")]` at each field that must carry it (the stored credential,
/// an issued token, a typed sheet answer, `clients.toml`).
///
/// # Errors
/// Whatever `serializer` returns when it cannot write the text.
pub fn write_secret<S: serde::Serializer>(
    secret: &SecretText,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(secret.expose())
}

/// [`write_secret`] for an optional secret.
///
/// # Errors
/// Whatever `serializer` returns when it cannot write the value.
pub fn write_secret_option<S: serde::Serializer>(
    secret: &Option<SecretText>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match secret {
        Some(secret) => serializer.serialize_some(secret.expose()),
        None => serializer.serialize_none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    /// `SecretText` has no `Serialize`: a writer names `write_secret`. (Method resolution picks
    /// the inherent constant only for a type that is `Serialize`.)
    #[test]
    fn a_secret_text_does_not_implement_serialize() {
        struct Probe<T>(std::marker::PhantomData<T>);
        trait Otherwise {
            const SERIALIZE: bool = false;
        }
        impl<T> Otherwise for Probe<T> {}
        impl<T: Serialize> Probe<T> {
            const SERIALIZE: bool = true;
        }
        const { assert!(!Probe::<SecretText>::SERIALIZE) };
        const { assert!(Probe::<String>::SERIALIZE) };
    }

    #[test]
    fn secrets_compare_by_their_text() {
        assert_eq!(SecretText::new("pw"), SecretText::new("pw"));
        assert_ne!(SecretText::new("pw"), SecretText::new("pw2"));
        assert_ne!(SecretText::new("pw"), SecretText::new("px"));
    }

    /// `SecretText` wipes itself when dropped: it is `ZeroizeOnDrop`, and `zeroize`, which its
    /// `Drop` calls, leaves nothing of the secret.
    #[test]
    fn a_secret_is_wiped_when_dropped() {
        fn wiped_on_drop<T: ZeroizeOnDrop>() {}
        wiped_on_drop::<SecretText>();
        let mut secret = SecretText::new("pw-xyz");
        secret.zeroize();
        assert!(
            secret.expose().bytes().all(|b| b == 0),
            "{:?}",
            secret.expose()
        );
    }
}
