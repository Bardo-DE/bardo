//! The tier rule: a typed action first, a hook (D-Bus, portal, MPRIS) second, computer use last
//! and only where the person turned it on for the app.

use serde::{Deserialize, Serialize};

/// How the companion reaches something.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// A typed action from the app's manifest.
    Typed,
    /// A hook the platform offers (D-Bus, a portal, MPRIS).
    Hook,
    /// Pixels: `org.bardo.ComputerUse` through `cua.run.start`.
    Cua,
}

/// What reaches an app: for each tier, whether it is there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Availability {
    /// A typed action covers the goal.
    pub typed: bool,
    /// A hook covers it.
    pub hook: bool,
    /// Computer use is on for the app (`cua.enabled`, off by default).
    pub cua: bool,
}

/// The first tier that is there, in order; none when nothing reaches the app (the answer is
/// `RefusalWire::NoWay`).
pub fn choose_tier(available: &Availability) -> Option<Tier> {
    [
        (Tier::Typed, available.typed),
        (Tier::Hook, available.hook),
        (Tier::Cua, available.cua),
    ]
    .into_iter()
    .find_map(|(tier, offered)| offered.then_some(tier))
}
