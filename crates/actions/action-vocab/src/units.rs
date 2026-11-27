//! Units: every number that means something has a type. Instants and counts are
//! account-core's (`UnixSeconds`, `Count`, `Permille`); these are the ones the actions service adds.

use serde::{Deserialize, Serialize};

macro_rules! unit {
    ($(#[$doc:meta])* $name:ident($inner:ty)) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub $inner);
    };
}

unit!(
    /// A length of time in whole seconds (a confirmation's expiry, a wall budget).
    Seconds(u32)
);
unit!(
    /// A length of time in milliseconds (a reviewer's latency, a stage timeout).
    Millis(u32)
);
impl Millis {
    /// The length as a [`Duration`](std::time::Duration), for a timer.
    #[must_use]
    pub fn duration(self) -> std::time::Duration {
        std::time::Duration::from_millis(u64::from(self.0))
    }
}

unit!(
    /// A length of time in whole days (how long a restore point is kept).
    Days(u32)
);
unit!(
    /// A number of characters (a text limit, a size shown to a reviewer).
    CharCount(u32)
);
unit!(
    /// How deep a follow-up chain is.
    Depth(u8)
);
unit!(
    /// The generation of a search: a newer query supersedes an older one.
    Generation(u64)
);
unit!(
    /// Digits after the point of a [`Decimal`](crate::Decimal).
    Scale(u8)
);
