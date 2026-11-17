//! The id of one of the person's helpers (a persistent named agent). It lives here, not in prov,
//! because both prov (`AgentRole::Helper`) and inference-core (`Asker::Helper`) name it, and
//! inference-core may depend only on account-core and computer-use-action.

use base_ids::IdError;
use serde::{Deserialize, Serialize};

/// One of the person's helpers. Opaque to the accounts service (the actions service mints and owns it): 1 to 128 bytes, no
/// control characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct HelperId(String);

impl HelperId {
    /// The id written as `text`, or why it is not one.
    ///
    /// # Errors
    ///
    /// `IdError` when `text` is empty, over 128 bytes, or holds a control character.
    pub fn parse(text: &str) -> Result<Self, IdError> {
        if !text.is_empty() && text.len() <= 128 && !text.chars().any(char::is_control) {
            Ok(Self(text.to_owned()))
        } else {
            Err(IdError::malformed("helper id", text))
        }
    }
}

base_ids::string_newtype!(HelperId, IdError);
