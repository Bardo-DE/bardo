//! The shell's own surfaces and the keyboard. The profile follows the focused toplevel, but bardo-shell's
//! surfaces (the launcher, menus, the switcher, the companion, the lock) are layer surfaces and
//! popups the compositor never lists as toplevels: while one holds the keyboard keycap would keep
//! the profile of the window underneath, usually `default`, which writes Command as Ctrl, and the
//! shell resolves Command as Super. So while a surface of the shell holds the keyboard the
//! profile is `keymap-aware`, above whatever the toplevel wants. Pure.

use std::collections::BTreeSet;

use key_remap_proto::ProfileName;

use super::source::keymap_aware_profile;

/// Whether the keyboard is on one of the shell's own surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShellFocus {
    /// A window of an app has it, or nobody does.
    #[default]
    Elsewhere,
    /// One or more surfaces of the shell hold it.
    Held,
}

/// Which of the shell's surfaces hold the keyboard, by the host's surface number.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShellKeys {
    holders: BTreeSet<u64>,
}

impl ShellKeys {
    /// These holders after `surface` gained the keyboard (`gained`) or lost it.
    #[must_use]
    pub fn after(mut self, surface: u64, gained: bool) -> ShellKeys {
        if gained {
            self.holders.insert(surface)
        } else {
            self.holders.remove(&surface)
        };
        self
    }

    /// These holders after `surface` closed: a surface that goes away holding the keyboard may
    /// never say it lost it.
    #[must_use]
    pub fn closed(self, surface: u64) -> ShellKeys {
        self.after(surface, false)
    }

    /// Whether any surface of the shell holds the keyboard.
    #[must_use]
    pub fn focus(&self) -> ShellFocus {
        if self.holders.is_empty() {
            ShellFocus::Elsewhere
        } else {
            ShellFocus::Held
        }
    }
}

/// The profile keycap is to have: `keymap-aware` while the shell holds the keyboard, else the
/// one the focused window wants.
#[must_use]
pub fn under_shell(shell: ShellFocus, window: ProfileName) -> ProfileName {
    match shell {
        ShellFocus::Held => keymap_aware_profile(),
        ShellFocus::Elsewhere => window,
    }
}
