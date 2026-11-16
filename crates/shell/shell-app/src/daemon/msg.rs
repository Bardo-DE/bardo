//! The daemon's host message: what reaches `Host::run`'s event handler from other threads.

use std::sync::mpsc::SyncSender;

use shell_ipc::request::Request;
use shell_ipc::response::Response;
use shell_settings::ShellFile;
use ui_settings::{Environment, UserStyle};

/// Which of the person's two `style.css` files a text is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sheet {
    /// `~/.config/bardo/style.css`, every quire app's.
    Desktop,
    /// `~/.config/bardo/shell/style.css`, the shell's own.
    App,
}

/// One message to the host thread (`HostHandle::send`).
#[derive(Debug)]
pub enum ShellMsg {
    /// An IPC request; answer on `reply` within the IPC server's timeout.
    Ipc {
        request: Request,
        reply: SyncSender<Response>,
    },
    /// An action a surface or a corner asked for (`crate::actions::spawn_asks` sends it).
    Action(shell_ui_kit::links::Asked),
    /// `settings.toml` settled on a new value.
    Settings(Box<ShellFile>),
    /// A `style.css` settled on a new text (empty when the file is gone).
    Style(Sheet, Box<UserStyle>),
    /// `extensions.toml` or an Extension folder settled on a change: the look is looked for again
    /// (the look watch in `settings::look` sends it).
    Look,
    /// The person pressed Turn On Again on the bar's notice (`settings::spawn_turn_on_follower` sends it).
    TurnOnStyles,
    /// The appearance (`appearance.toml` and the portal) settled on a new value.
    Appearance(Box<Environment>),
    /// Hold or drop layer-shell-host's session lock (`crate::lock::spawn_follower` sends it).
    Lock(crate::lock::LockWish),
    /// Map or unmap the idle dim surfaces (`crate::idle_dim::spawn_follower` sends it).
    Dim(shell_model::idle::DimShown),
    /// A source of questions changed; the sheets' latch looks again (`crate::sheet_latch`).
    Sheets,
    /// Open, switch or close the launcher for a summon (`crate::companion::spawn_follower`
    /// sends it).
    Summon(shell_model::companion::SummonOut),
    /// One note of the desktop's own compositor's shell protocol (`compositor::ShellSeat`'s
    /// forwarder sends it).
    Note(shell_model::desktop::ShellNote),
    /// Where the docks' icons rest now (`targets::forward` sends it).
    DockRects(shell_model::desktop::target::OutputRects),
    /// A command for the compositor's arm (`crate::outgoing` sends a pick of a
    /// window's menu).
    Command(shell_model::desktop::DesktopCommand),
    /// The desktop's view changed (`targets::nudge` sends it); the seat reads the view it holds.
    Desktop,
}
