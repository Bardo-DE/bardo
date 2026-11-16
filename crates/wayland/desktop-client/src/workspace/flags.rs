//! Workspace state and abilities.

use std::collections::BTreeSet;

/// A workspace state bit (`ext_workspace_handle_v1.state`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WorkspaceFlag {
    /// The workspace on show.
    Active,
    /// Wants attention.
    Urgent,
    /// Not to be listed.
    Hidden,
}

/// What a workspace lets the client request (`ext_workspace_handle_v1.workspace_capabilities`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WorkspaceAbility {
    /// `activate`.
    Activate,
    /// `deactivate`.
    Deactivate,
    /// `remove`.
    Remove,
    /// `assign` to another group.
    Assign,
}

/// A set of values of an ordered kind; one type for flags and abilities.
pub(crate) type Set<T> = BTreeSet<T>;

/// The values of `table` whose bit is set in `bits`; unknown bits are skipped.
fn set_of<T: Copy + Ord>(table: &[(u32, T)], bits: u32) -> Set<T> {
    table
        .iter()
        .filter(|(bit, _)| bits & bit != 0)
        .map(|(_, value)| *value)
        .collect()
}

impl WorkspaceFlag {
    /// The states set in a `state` bitfield; unknown bits are skipped.
    #[must_use]
    pub fn set_from_bits(bits: u32) -> Set<Self> {
        set_of(
            &[(1, Self::Active), (2, Self::Urgent), (4, Self::Hidden)],
            bits,
        )
    }
}

impl WorkspaceAbility {
    /// The abilities set in a `workspace_capabilities` bitfield; unknown bits are skipped.
    #[must_use]
    pub fn set_from_bits(bits: u32) -> Set<Self> {
        set_of(
            &[
                (1, Self::Activate),
                (2, Self::Deactivate),
                (4, Self::Remove),
                (8, Self::Assign),
            ],
            bits,
        )
    }
}

/// Whether a `group_capabilities` bitfield lets the client create a workspace; unknown bits are
/// skipped.
#[must_use]
pub fn can_create_workspace(bits: u32) -> bool {
    bits & 1 != 0
}

/// The coordinates in a wire `coordinates` array (native-endian `u32`s); a trailing partial
/// word is skipped.
#[must_use]
pub fn coordinates_from_array(bytes: &[u8]) -> Vec<u32> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|w| u32::from_ne_bytes(*w))
        .collect()
}
