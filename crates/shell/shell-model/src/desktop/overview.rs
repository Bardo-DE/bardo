//! The requests that open the desktop's overviews from the shell's own verbs.

use desktop_protocols::vocab::{OverviewMode, Toggle};

use super::DesktopCommand;

/// Open Mission Control (`bardo-shell workspace overview`, the hot corner that shows the Spaces).
#[must_use]
pub fn open_mission_control() -> DesktopCommand {
    DesktopCommand::Overview {
        mode: OverviewMode::MissionControl,
        toggle: Toggle::On,
    }
}

/// Open App Exposé, the focused app's windows spread out (the `app_windows` hot corner).
#[must_use]
pub fn open_app_expose() -> DesktopCommand {
    DesktopCommand::Overview {
        mode: OverviewMode::AppExpose,
        toggle: Toggle::On,
    }
}
