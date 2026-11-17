//! The default key bindings (C30: the compositor's settings document carries them; the shell ships none).

use crate::input::Bindings;

/// The bindings a fresh settings document holds: none yet (FINDINGS: the default table).
pub(crate) fn defaults() -> Bindings {
    Bindings(Vec::new())
}
