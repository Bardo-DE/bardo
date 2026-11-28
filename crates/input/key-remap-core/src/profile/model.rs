//! `ModMap` and `Profile`: one flat, fully resolved profile.

use std::collections::BTreeMap;

use super::action::RuleAction;
use super::chord::Trigger;
use crate::role::Role;
use key_types::Modifier;

/// The modifier each role writes when it is pressed. Shift always writes Shift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModMap {
    pub command: Modifier,
    pub option: Modifier,
    pub control: Modifier,
}

impl ModMap {
    /// Each role writes the modifier it is named after: no remapping.
    pub const IDENTITY: ModMap = ModMap {
        command: Modifier::Super,
        option: Modifier::Alt,
        control: Modifier::Ctrl,
    };

    /// What `role` writes when pressed.
    #[must_use]
    pub const fn out(self, role: Role) -> Modifier {
        match role {
            Role::Command => self.command,
            Role::Option => self.option,
            Role::Control => self.control,
            Role::Shift => Modifier::Shift,
        }
    }
}

/// A profile with inheritance already applied: every rule it has, inherited ones included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub modmap: ModMap,
    pub rules: BTreeMap<Trigger, RuleAction>,
}
