//! The private `quire_*` protocols served to the shell and the agent tier (P2-Z.
//!
//! Behind the crate's `quire` feature, A8): `bardo_desktop_manager_v1` (the desktop model of apps,
//! windows and Spaces) and `bardo_shell_v1` (the shell's surface roles and the events only the
//! compositor sees). `bardo_window_manager_v1`, which every app may use, stays beside the standard
//! protocols in [`crate::quire_window`].
//!
//! Each `Part::new` creates its global. The I7 rows decide who sees it
//! (`compositor_core::policy::advertised`): the shell sees both, the agent tier sees the
//! desktop manager (and is refused its management requests), the public socket and
//! sandboxes see neither.

pub mod desktop;
pub mod shell;
pub mod shell_overview;
pub mod shell_permission;
pub mod shell_screen;
pub mod shell_space;
pub mod shell_split;
pub mod shell_trial;
pub mod space_shot;

use smithay::reexports::wayland_server::WEnum;

/// The wire code of a request's enum argument (`None` for a value the XML does not name).
pub(crate) fn code_of<T: Into<u32>>(value: WEnum<T>) -> Option<u32> {
    match value {
        WEnum::Value(value) => Some(value.into()),
        WEnum::Unknown(_) => None,
    }
}
