//! What the person typed for a network's password.

use base_secret::SecretText;

/// The fewest characters a WPA password has.
const SHORTEST: usize = 8;

/// A network password. Its `Debug` never prints the text, so no log line of an input or an effect
/// that carries one can leak it. It is not trimmed: a password may begin or end with a space.
#[derive(Clone, PartialEq, Eq)]
pub struct Passphrase(SecretText);

impl Passphrase {
    /// The password `text` is.
    #[must_use]
    pub fn new(text: &str) -> Passphrase {
        Passphrase(SecretText::new(text))
    }

    /// Whether it is long enough to be a WPA password; the sheet's Join waits for this.
    #[must_use]
    pub fn is_long_enough(&self) -> bool {
        self.0.expose().chars().count() >= SHORTEST
    }

    /// The text, for the one place that sends it to the network service.
    #[must_use]
    pub fn into_text(self) -> String {
        self.0.expose().to_owned()
    }
}

impl std::fmt::Debug for Passphrase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Passphrase(<hidden>)")
    }
}
