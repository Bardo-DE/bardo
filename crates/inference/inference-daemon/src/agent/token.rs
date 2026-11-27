//! The per-session secret an agent sends as its "API key".
//!
//! It is not the provider's key: it only lets a process that was told it reach the session's
//! loopback listener. Random (32 bytes from the kernel), compared in constant time, and printed by
//! nothing: `Debug` shows its length.

use subtle::ConstantTimeEq;

/// The prefix of every token, so a person who sees one in a process list knows what it is.
const PREFIX: &str = "sk-account-agent-";

/// A session's token.
#[derive(Clone, PartialEq, Eq)]
pub struct Token(String);

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Token(<{} bytes>)", self.0.len())
    }
}

impl Token {
    /// A fresh token.
    ///
    /// # Errors
    /// [`base_random::RandomError`] when the kernel gives no randomness.
    pub fn fresh() -> Result<Self, base_random::RandomError> {
        Ok(Self(format!("{PREFIX}{}", base_random::hex::<32>()?)))
    }

    /// The token as text, for the one reply to the launcher and for tests.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Whether `presented` is this token. Takes the same time for every input of the same
    /// length, and the length is not a secret.
    #[must_use]
    pub fn matches(&self, presented: &str) -> bool {
        self.0.as_bytes().ct_eq(presented.as_bytes()).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_fresh_long_and_hidden() {
        let (a, b) = (Token::fresh().expect("a"), Token::fresh().expect("b"));
        assert_ne!(a, b);
        assert_eq!(a.expose().len(), PREFIX.len() + 64);
        assert!(!format!("{a:?}").contains(a.expose()));
    }

    #[test]
    fn only_the_whole_token_matches() {
        let token = Token::fresh().expect("token");
        let own = token.expose().to_owned();
        let cases = [
            (own.clone(), true),
            (String::new(), false),
            (format!("{own}x"), false),
            (own[..own.len() - 1].to_owned(), false),
            (own.replace(&own[own.len() - 1..], "!"), false),
            (own.to_uppercase(), false),
        ];
        for (presented, want) in cases {
            assert_eq!(token.matches(&presented), want, "{presented:?}");
        }
    }
}
