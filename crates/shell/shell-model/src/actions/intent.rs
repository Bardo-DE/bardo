//! Which action of `org.bardo.Shell` (`dist/intents/org.bardo.Shell.toml`) does what an action
//! of the shell does. The match has no wildcard, so a new `ActionId` is a compile error until it
//! is placed; a test holds the names to the manifest, so the file the actions service reads and the actions
//! the shell has cannot drift apart.

use shell_launcher::actions::ActionId;

/// The manifest action that asks for `id`, where the manifest declares one. Several actions
/// can share one (a screenshot's kind, a workspace's direction are parameters there).
#[must_use]
pub fn intent_of(id: ActionId) -> Option<&'static str> {
    use ActionId as A;
    match id {
        A::Lock => Some("shell.lock"),
        A::Launcher => Some("shell.launcher.open"),
        A::NextSpace | A::PreviousSpace => Some("shell.workspace.switch"),
        A::ScreenshotArea | A::ScreenshotWindow | A::ScreenshotScreen => Some("shell.screenshot"),
        A::PinApp => Some("shell.dock.pin"),
        A::UnpinApp => Some("shell.dock.unpin"),
        A::LogOut
        | A::Restart
        | A::ShutDown
        | A::Sleep
        | A::EmojiAndSymbols
        | A::NextInputSource
        | A::PowerMenu
        | A::ForceQuitPanel
        | A::NotificationCenter
        | A::ControlCenter
        | A::ShowDesktop
        | A::MissionControl
        | A::NextWindow
        | A::SwitcherNext
        | A::SwitcherPrev
        | A::SwitcherCommit
        | A::SwitcherCancel
        | A::OpenSettings
        | A::OpenApp
        | A::ShowInFiles
        | A::HideApp
        | A::QuitApp
        | A::ForceQuitApp
        | A::ToggleDockHide
        | A::ToggleDockMagnification
        | A::EmptyTrash
        | A::ToggleFocus
        | A::Reload => None,
    }
}
