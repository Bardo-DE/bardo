//! Consent decisions as pure functions: what the stored grants say for one key, and what an
//! app may learn about a need without holding a grant.

use super::grant::{Decision, Grant, GrantScope};
use crate::GrantId;
use serde::{Deserialize, Serialize};

/// What the consent store says for one key.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Verdict {
    /// Allowed by this grant, for this long.
    Granted {
        /// The grant that allows it.
        grant: GrantId,
        /// Once (spent by this use), always, or for a session.
        scope: GrantScope,
    },
    /// Refused; the app is not prompted again.
    Denied,
    /// Nothing decided yet; the user must be asked.
    Ask,
}

/// The newest grant for exactly this key decides; on equal times a denial wins.
pub fn decide<K: Eq>(grants: &[Grant<K>], key: &K) -> Verdict {
    let newest = grants
        .iter()
        .filter(|g| g.key == *key)
        .max_by(|a, b| a.at.cmp(&b.at).then_with(|| a.decision.cmp(&b.decision)));
    match newest {
        None => Verdict::Ask,
        Some(grant) if grant.decision == Decision::Deny => Verdict::Denied,
        Some(grant) => Verdict::Granted {
            grant: grant.id.clone(),
            scope: grant.scope.clone(),
        },
    }
}

/// What an app may learn about a need without holding a grant: no identities (design/31 §4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    /// The app already holds a grant for a fitting account.
    Granted,
    /// A fitting account exists; the app must ask (`Choose`).
    AvailableNeedsConsent,
    /// Every fitting account was refused to this app.
    Denied,
    /// No account fits, but one could be added.
    NeedsAccount,
    /// No provider can serve this need.
    Unsupported,
}

/// The availability for one app, from the verdicts of the accounts whose offer fits.
/// `offered` is whether any installed provider declares a capability that meets the need, so
/// adding an account helps.
pub fn availability(fitting: &[Verdict], offered: bool) -> Availability {
    let any = |want: fn(&Verdict) -> bool| fitting.iter().any(want);
    if any(|v| matches!(v, Verdict::Granted { .. })) {
        Availability::Granted
    } else if any(|v| matches!(v, Verdict::Ask)) {
        Availability::AvailableNeedsConsent
    } else if !fitting.is_empty() {
        Availability::Denied
    } else if offered {
        Availability::NeedsAccount
    } else {
        Availability::Unsupported
    }
}

#[cfg(test)]
mod tests;
