//! Units: every number that means something has a type.

use serde::{Deserialize, Serialize};

pub use model_http::WaitMs;

macro_rules! unit {
    ($(#[$doc:meta])* $name:ident($inner:ty)) => {
        $(#[$doc])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub $inner);
    };
}

unit!(
    /// A number of model tokens.
    Tokens(u32)
);
unit!(
    /// Thousandths: a sampling temperature of 0.7 is `Milli(700)`.
    Milli(u16)
);
unit!(
    /// A number of images.
    ImageCount(u16)
);
unit!(
    /// Seconds a server asks a client to wait before retrying.
    RetrySeconds(u32)
);
unit!(
    /// The position of one tool call among those a turn makes.
    CallIndex(u16)
);
unit!(
    /// A plain count of things (documents, list items, a top-k).
    Count(u32)
);
unit!(
    /// A random seed. 32 bits, so it fits a TOML integer.
    Seed(u32)
);
unit!(
    /// The length of an embedding vector.
    Dims(u32)
);
unit!(
    /// The most texts one embedding call takes.
    BatchMax(u32)
);
unit!(
    /// A duration in seconds.
    Seconds(u32)
);
unit!(
    /// Thousandths of a whole, for a jitter.
    Permille(u16)
);
unit!(
    /// The number of the try in a retry loop, from 1.
    Attempt(u8)
);
unit!(
    /// How many repair prompts a session may spend on a reply that does not fit. A setting the
    /// daemon reads; no default lives here.
    RepairBudget(u8)
);
unit!(
    /// A server's slot number (llama-server's `id_slot`).
    SlotId(u16)
);
unit!(
    /// A length in characters.
    CharCount(u32)
);
unit!(
    /// An HTTP status a server answered with, kept as a number (this crate does not depend on
    /// `model-http`; the wire crate converts).
    ServerStatus(u16)
);

/// A setting that is either left to the engine (or the model's catalog default) or set to a value.
///
/// Written out in full in the catalog and on the wire (an `Option` would let a file leave a field
/// out unnoticed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Knob<T> {
    /// The engine's own default.
    #[default]
    Off,
    /// This value.
    Set(T),
}

impl<T> Knob<T> {
    /// Whether the knob is left to the default.
    pub fn is_off(&self) -> bool {
        matches!(self, Knob::Off)
    }
}
