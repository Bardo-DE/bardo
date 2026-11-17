//! What a client asks the daemon: one [`Request`] per connection.

use serde::{Deserialize, Serialize};
use shell_launcher::AppId;
use shell_model::actions::Ask;
use shell_model::intents::IntentsAsk;
use shell_model::key_remap::KeyRemapAsk;

use crate::act::DisplayRequest;
use crate::debug::DebugRequest;
use crate::show::{CompanionRequest, LauncherRequest, Reveal, VoiceRequest};
use crate::status::StatusRequest;

/// One request. A connection carries exactly one, then its [`crate::Response`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Request {
    /// Is the daemon alive? Answered with `Response::Pong`.
    Ping,
    /// Close every surface and exit the daemon.
    Quit,
    /// Re-read the settings without restarting.
    Reload,
    /// Do an action of the shell: lock, a screenshot, a panel, a step of the switcher, a pin
    /// (`bardo-shell lock`, `bardo-shell screenshot`, `bardo-shell launcher toggle`, ...). One request for every
    /// action; what each is lives in `shell_model::actions`.
    Do(Ask),
    /// Open, close or seed the launcher (`bardo-shell launcher {open,close}`; `toggle` is an action).
    Launcher(LauncherRequest),
    /// The power menu, opened, closed or flipped (`bardo-shell power-menu {open,close,toggle}`). An
    /// action only opens or flips a panel, so a close needs its own word.
    PowerMenu(Reveal),
    /// The Force Quit Applications panel (`bardo-shell force-quit {open,close,toggle}`), on the output
    /// of the active window.
    ForceQuit(Reveal),
    /// The notification center (`bardo-shell notifications {open,close}`).
    NotificationCenter(Reveal),
    /// The companion's summon (`bardo-shell companion {tap,summon,dismiss,stop-all}`).
    Companion(CompanionRequest),
    /// Hold-to-talk (`bardo-shell voice {hold-start,hold-end,hold-cancel}`).
    Voice(VoiceRequest),
    /// Read a state: answered with `Response::Status`.
    Status(StatusRequest),
    /// The brightness and volume keys (`bardo-shell display {brightness,volume} ...`), bound to
    /// `XF86MonBrightness`*/`XF86Audio`* in the compositor's shortcuts.
    Display(DisplayRequest),
    /// Forget every clipboard entry now (`bardo-shell clipboard clear`).
    ClearClipboard,
    /// Dismiss every banner on screen (each sender hears `NotificationClosed`, reason 2).
    DismissNotifications,
    /// Give the keyboard to the newest transient card: the screenshot thumbnail or the newest
    /// banner, whichever appeared last (`bardo-shell focus-latest`; bound in the compositor's shortcuts
    /// like the launcher's). Nothing happens when none is showing.
    FocusLatest,
    /// Restart the key remap daemon, or end its pause (`bardo-shell key-remap retry|resume`).
    KeyRemap(KeyRemapAsk),
    /// Withdraw a standing terminal grant, or let a paused terminal go on (`bardo-shell intents
    /// revoke|resume`).
    Intents(IntentsAsk),
    /// Open an installed app the way a click in the dock does (`bardo-shell launch <desktop-id>`):
    /// answered `Response::Launched` once the app's process exists, or `Failed`.
    Launch(AppId),
    /// Development probes (`dev/accept/*.sh`); answered even in a release build, acted on only
    /// when the daemon was built with the `debug` feature.
    Debug(DebugRequest),
}
