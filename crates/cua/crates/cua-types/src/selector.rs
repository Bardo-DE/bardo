//! Finding nodes again by role and name, for procedures (a later wave). The types are frozen so
//! `SuccessCheck` can name them.

use crate::role::Role;
use serde::{Deserialize, Serialize};

/// How a step matches a node's name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum NameMatch {
    /// Any name.
    Any,
    /// This name exactly.
    Exact(String),
    /// The value of the named slot of a procedure: a parameter filled at replay, never a stored
    /// value.
    Slot(String),
}

/// One step down the tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectorStep {
    /// The role to match.
    pub role: Role,
    /// The name to match.
    pub name: NameMatch,
}

/// A path of steps from the window root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeSelector {
    /// The steps.
    pub path: Vec<SelectorStep>,
}
