//! A switch's state as the toggle draws it and reports it.

use ds::prelude::*;

/// How the toggle draws `on`.
pub(super) fn check_of(on: bool) -> Check {
    if on { Check::On } else { Check::Off }
}

/// Whether the toggle's new value asks for on.
pub(super) fn switch_of(check: Check) -> bool {
    check == Check::On
}
