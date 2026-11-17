//! `bardo-shell debug ...`: the development probes' command line, and their pure mappings to requests.
//! Each made-up input is built as the `shell-model` value the service would have received.

use std::path::PathBuf;

use base_words::Word;
use clap::Subcommand;
use shell_ipc::debug::{DebugRequest, KeyAction, KeySpec, Point, PointerButton, SurfaceName};
use shell_ipc::request::Request;

use shell_launcher::AppId;
use shell_model::auth::FakeVerdict;
use shell_model::companion::{Mode, SessionId};
use shell_model::hot_corners::Corner;
use shell_model::osd::OsdKind;
use shell_model::units::Percent;
use shell_model::volume::Mute;

use super::made_up::{NotifyArgs, PolkitArgs, polkit_request, posting};
use super::word::word;

/// `bardo-shell debug ...`.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum DebugAction {
    /// Frame statistics for every surface.
    Stats,
    /// Move the pointer to X Y on SURFACE (`shell-dock-DP-1`).
    Pointer { surface: String, x: i32, y: i32 },
    /// The pointer leaves SURFACE.
    Leave { surface: String },
    /// Render SURFACE to a PNG at PATH.
    Capture { surface: String, path: PathBuf },
    /// Press and release a pointer button at X Y on SURFACE (`shell-bar-DP-1`).
    Click {
        surface: String,
        x: i32,
        y: i32,
        /// Which button.
        #[arg(long, value_parser = word::<PointerButton>(), default_value = PointerButton::Primary.slug())]
        button: PointerButton,
    },
    /// Press a key on SURFACE as if typed (`shell-power-menu-DP-1`): KEY is a W3C key name or
    /// `Space`, then `+shift`, `+ctrl`, `+alt` or `+super` (`Tab`, `ArrowRight`, `k+ctrl`).
    Key {
        surface: String,
        key: String,
        /// Press and release (`tap`), or hold, or let go.
        #[arg(long, value_parser = word::<KeyAction>(), default_value = KeyAction::Tap.slug())]
        action: KeyAction,
    },
    /// Show the OSD for a made-up change, moving no real level.
    Osd {
        /// What changed.
        #[arg(value_parser = word::<OsdKind>())]
        topic: OsdKind,
        /// The level to show, 0-100.
        percent: u8,
        /// Whether the made-up volume is muted.
        #[arg(long, value_parser = word::<Mute>(), default_value = Mute::Audible.slug())]
        mute: Mute,
    },
    /// Post a made-up notification without D-Bus: a banner, and a row in the center.
    Notify(NotifyArgs),
    /// Show the screenshot thumbnail for PATH, as if it had just been saved.
    Screenshot { path: PathBuf },
    /// Carry out CORNER's hot corner action, as if the pointer had rested there.
    Corner {
        #[arg(value_parser = word::<Corner>())]
        corner: Corner,
    },
    /// `button` presses at X Y on SURFACE and stays down: the first half of a real drag
    /// (layer-shell-host's `Inject::PointerDown`). `bardo-shell debug pointer` moves while it
    /// is held, and `pointer-release` lets go.
    PointerPress {
        surface: String,
        x: i32,
        y: i32,
        /// Which button.
        #[arg(long, value_parser = word::<PointerButton>(), default_value = PointerButton::Primary.slug())]
        button: PointerButton,
    },
    /// `button` releases at X Y on SURFACE: the second half of a real drag
    /// (`Inject::PointerUp`).
    PointerRelease {
        surface: String,
        x: i32,
        y: i32,
        /// Which button.
        #[arg(long, value_parser = word::<PointerButton>(), default_value = PointerButton::Primary.slug())]
        button: PointerButton,
    },
    /// Feed the lock screen's password check a made-up verdict (a debug daemon only).
    Auth {
        #[arg(value_parser = word::<FakeVerdict>())]
        verdict: FakeVerdict,
    },
    /// Show a made-up polkit request (its stand-in helper accepts `shell-fake`).
    Polkit(PolkitArgs),
    /// Open the app switcher on made-up apps (desktop ids, the current one first).
    Switcher {
        #[arg(required = true)]
        apps: Vec<String>,
    },
    /// Print the open launcher's sections, rows and preview (a debug daemon only).
    LauncherRows,
    /// Wake idle as real input would (a debug daemon only; inside the lock grace it opens the
    /// lock).
    IdleWake,
    /// Switch the conversation SESSION to MODE (`auto` or `manual`) as the shell does, for a run
    /// the companion's box does not hold (one asked through `bardo-do`). The router's
    /// `ModeChanged` signal says it took; a refusal is in the daemon's log.
    Mode {
        /// The conversation's id, as `SessionOpened` names it.
        #[arg(value_parser = session_id)]
        session: SessionId,
        /// `auto` or `manual`.
        #[arg(value_parser = mode_word)]
        mode: Mode,
    },
}

/// The request `bardo-shell debug ACTION` sends.
pub fn debug_request(action: &DebugAction) -> Request {
    Request::Debug(match action {
        DebugAction::Stats => DebugRequest::Stats,
        DebugAction::Pointer { surface, x, y } => DebugRequest::Pointer {
            surface: surface_name(surface),
            at: point(*x, *y),
        },
        DebugAction::Leave { surface } => DebugRequest::Leave {
            surface: surface_name(surface),
        },
        DebugAction::Capture { surface, path } => DebugRequest::Capture {
            surface: surface_name(surface),
            path: path.clone(),
        },
        DebugAction::Click {
            surface,
            x,
            y,
            button,
        } => DebugRequest::Click {
            surface: surface_name(surface),
            at: point(*x, *y),
            button: *button,
        },
        DebugAction::PointerPress {
            surface,
            x,
            y,
            button,
        } => DebugRequest::PointerPress {
            surface: surface_name(surface),
            at: point(*x, *y),
            button: *button,
        },
        DebugAction::PointerRelease {
            surface,
            x,
            y,
            button,
        } => DebugRequest::PointerRelease {
            surface: surface_name(surface),
            at: point(*x, *y),
            button: *button,
        },
        DebugAction::Key {
            surface,
            key,
            action,
        } => DebugRequest::Key {
            surface: surface_name(surface),
            key: KeySpec(key.clone()),
            action: *action,
        },
        DebugAction::Osd {
            topic,
            percent,
            mute,
        } => DebugRequest::Osd {
            topic: *topic,
            percent: Percent(u16::from(*percent)),
            mute: *mute,
        },
        DebugAction::Notify(args) => DebugRequest::Notify(Box::new(posting(args))),
        DebugAction::Screenshot { path } => DebugRequest::Screenshot { path: path.clone() },
        DebugAction::Corner { corner } => DebugRequest::Corner(*corner),
        DebugAction::Auth { verdict } => DebugRequest::Auth(*verdict),
        DebugAction::Polkit(args) => DebugRequest::Polkit(polkit_request(args)),
        DebugAction::Switcher { apps } => DebugRequest::Switcher {
            apps: apps.iter().cloned().map(AppId).collect(),
        },
        DebugAction::LauncherRows => DebugRequest::LauncherRows,
        DebugAction::IdleWake => DebugRequest::IdleWake,
        DebugAction::Mode { session, mode } => DebugRequest::Mode {
            session: session.clone(),
            mode: *mode,
        },
    })
}

fn session_id(text: &str) -> Result<SessionId, String> {
    SessionId::parse(text).map_err(|error| error.to_string())
}

fn mode_word(text: &str) -> Result<Mode, String> {
    match text {
        "auto" => Ok(Mode::Auto),
        "manual" => Ok(Mode::Manual),
        _ => Err("auto or manual".to_owned()),
    }
}

fn surface_name(surface: &str) -> SurfaceName {
    SurfaceName(surface.to_owned())
}

fn point(x: i32, y: i32) -> Point {
    Point { x, y }
}
