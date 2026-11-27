//! The ids agent-side crates share. Slug ids use the accounts service's one id grammar; entity names carry
//! their own.

use account_core::AppName;
use serde::{Deserialize, Serialize};

macro_rules! text_id {
    ($(#[$doc:meta])* $name:ident, $what:literal, $ok:expr) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, ::serde::Serialize, ::serde::Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            /// The id written as `text`, or why it is not one.
            ///
            /// # Errors
            ///
            /// Returns [`CoreError::MalformedId`](::account_core::CoreError::MalformedId) when
            /// `text` is not a valid id.
            pub fn parse(text: &str) -> Result<Self, ::account_core::CoreError> {
                let ok: fn(&str) -> bool = $ok;
                if ok(text) {
                    Ok(Self(text.to_owned()))
                } else {
                    Err(::account_core::CoreError::MalformedId { what: $what, text: text.to_owned() })
                }
            }
        }

        base_ids::string_newtype!($name, ::account_core::CoreError);
    };
}

pub(crate) use text_id;

/// Two or more dot-separated elements, each `[a-z][a-z0-9_]*`.
fn dotted(text: &str) -> bool {
    let element = |e: &str| {
        let mut bytes = e.bytes();
        bytes.next().is_some_and(|b| b.is_ascii_lowercase())
            && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    };
    let mut count = 0;
    let all = text.split('.').all(|e| {
        count += 1;
        element(e)
    });
    all && count >= 2
}

/// 1 to 128 bytes, no control characters: an id prov does not interpret.
fn opaque(text: &str) -> bool {
    !text.is_empty() && text.len() <= 128 && !text.chars().any(char::is_control)
}

base_ids::slug_id!(
    /// One companion session. A slug minted by the companion service.
    SessionId,
    "session id"
);
base_ids::slug_id!(
    /// One computer-use run. A slug minted by the computer-use daemon (`r-<n>`).
    RunId,
    "run id"
);
text_id!(
    /// An external MCP client's name, as it introduced itself: 1 to 64 bytes, no control
    /// characters.
    ClientName,
    "client name",
    |t| !t.is_empty() && t.len() <= 64 && !t.chars().any(char::is_control)
);
text_id!(
    /// What kind of thing an app owns: two or more dotted `[a-z][a-z0-9_]*` elements
    /// (`mail.thread`).
    EntityKind,
    "entity kind",
    dotted
);
text_id!(
    /// An app-local key for one entity: opaque, 1 to 512 bytes, no control characters.
    EntityKey,
    "entity key",
    |t| !t.is_empty() && t.len() <= 512 && !t.chars().any(char::is_control)
);
text_id!(
    /// A typed action's name (`mail.thread.archive`), the same grammar as [`EntityKind`].
    ActionName,
    "action name",
    dotted
);

base_ids::slug_id!(
    /// One task: an actions service `Session` for one prompt, worker or side conversation.
    ///
    /// A slug minted by the companion service. It moved here from action-core so
    /// [`AgentRole::Worker`] and [`AgentRef::Worker`] can name it.
    ///
    /// [`AgentRole::Worker`]: crate::AgentRole::Worker
    /// [`AgentRef::Worker`]: crate::AgentRef::Worker
    TaskId,
    "task id"
);
base_ids::slug_id!(
    /// One [`Message`](crate::Message). A slug minted by the router that stamps it.
    MessageId,
    "message id"
);
base_ids::slug_id!(
    /// A conversation: the messages that answer one another share it. A slug; the first
    /// message's id is a fine choice.
    ThreadId,
    "thread id"
);
text_id!(
    /// A reference to one action outcome, opaque to prov (the actions service owns the outcome and its
    /// handle): 1 to 128 bytes, no control characters.
    OutcomeRef,
    "outcome ref",
    opaque
);
text_id!(
    /// A reference to one undo journal entry, opaque to prov (the actions service owns `UndoId`): 1 to 128
    /// bytes, no control characters.
    UndoHandle,
    "undo handle",
    opaque
);

/// One of the person's helpers: a persistent named agent (account-core owns the type, because
/// inference-core names it too).
pub use account_core::HelperId;

/// One thing an app owns, as the router, memory and the confirmation sheet name it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EntityId {
    /// The app that owns it.
    pub app: AppName,
    /// What kind of thing it is.
    pub kind: EntityKind,
    /// Which one.
    pub key: EntityKey,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_follow_their_grammars() {
        let long_key = "k".repeat(513);
        let cases: Vec<(&str, bool, bool)> = vec![
            ("mail.thread", true, true),
            ("mail.thread.archive", true, true),
            ("mail", false, false),
            ("Mail.thread", false, false),
            ("mail..thread", false, false),
            ("mail.1thread", false, false),
            ("mail.thread_x9", true, true),
            ("", false, false),
            (long_key.as_str(), false, false),
        ];
        for (text, kind_ok, action_ok) in cases {
            assert_eq!(EntityKind::parse(text).is_ok(), kind_ok, "kind {text:?}");
            assert_eq!(
                ActionName::parse(text).is_ok(),
                action_ok,
                "action {text:?}"
            );
        }
        EntityKey::parse("a/b c").unwrap();
        EntityKey::parse("").unwrap_err();
        EntityKey::parse(&"k".repeat(513)).unwrap_err();
        EntityKey::parse("a\nb").unwrap_err();
        RunId::parse("r-17").unwrap();
        RunId::parse("R 17").unwrap_err();
        ClientName::parse("Claude Desktop").unwrap();
        ClientName::parse("").unwrap_err();
        assert!(TaskId::parse("t-3").is_ok() && TaskId::parse("T 3").is_err());
        assert!(MessageId::parse("m-1").is_ok() && MessageId::parse("").is_err());
        assert!(ThreadId::parse("m-1").is_ok() && ThreadId::parse("a b").is_err());
        OutcomeRef::parse("out:41/a").unwrap();
        OutcomeRef::parse("").unwrap_err();
        OutcomeRef::parse(&"o".repeat(129)).unwrap_err();
        UndoHandle::parse("u\n1").unwrap_err();
        assert!(HelperId::parse("01J9ZK").is_ok() && HelperId::parse("").is_err());
    }
}
