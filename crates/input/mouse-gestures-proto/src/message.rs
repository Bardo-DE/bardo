//! The messages each side sends, and the filter a subscriber narrows them by.

use serde::{Deserialize, Serialize};

use crate::vocab::{Axis, GestureId, HostAction, Phase, TapKind};

/// The mouse the mouse gestures daemon is reading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub path: String,
    pub vendor: u16,
    pub product: u16,
}

/// A kind of message a subscriber may ask for. `Hello` and `DeviceGone` always arrive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Topic {
    Scroll,
    Swipe,
    Tap,
    Touches,
}

/// What a subscriber receives.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Filter {
    #[default]
    All,
    Only(Vec<Topic>),
}

impl Filter {
    /// Whether `msg` passes this filter.
    #[must_use]
    pub fn admits(&self, msg: &ToHost) -> bool {
        match (self, msg.topic()) {
            (_, None) | (Filter::All, _) => true,
            (Filter::Only(topics), Some(t)) => topics.contains(&t),
        }
    }
}

/// The mouse gestures daemon to a host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ToHost {
    /// First on every connection, and again after the mouse reconnects.
    Hello { version: u16, device: DeviceInfo },
    /// One- or two-finger scroll (G1, G2): px in finger direction, gains applied.
    Scroll {
        gesture: GestureId,
        phase: Phase,
        dx_px: f32,
        dy_px: f32,
        fingers: u8,
        axis: Option<Axis>,
        t_ns: u64,
    },
    /// Two-finger horizontal travel in grid units (26 per mm; on the Magic Mouse 2 its raw
    /// units, as before), for a host running the live model (12.3.7).
    Swipe {
        gesture: GestureId,
        phase: Phase,
        fingers: u8,
        dx_units: f32,
        t_ns: u64,
    },
    /// A double-tap fired while this host held the pointer.
    Tap { kind: TapKind, t_ns: u64 },
    /// The first finger landed (`active`) or the last one lifted: fingers on the mouse or not, for
    /// the host's double-scroll window (11.3.14).
    Touches { active: bool },
    /// The mouse went away; every running gesture was Cancelled first.
    DeviceGone,
    /// A gesture's action that needs the person's session. Sent to the shell only (the connection
    /// the daemon believes about the lock), whatever its filter says; the shell carries it out.
    Run { action: HostAction },
}

impl ToHost {
    /// The topic a subscriber filters this message by; `None` for always-delivered messages.
    #[must_use]
    pub fn topic(&self) -> Option<Topic> {
        match self {
            Self::Hello { .. } | Self::DeviceGone | Self::Run { .. } => None,
            Self::Scroll { .. } => Some(Topic::Scroll),
            Self::Swipe { .. } => Some(Topic::Swipe),
            Self::Tap { .. } => Some(Topic::Tap),
            Self::Touches { .. } => Some(Topic::Touches),
        }
    }
}

/// A host to mouse_gestures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ToDaemon {
    /// Sent on `wl_pointer.enter`/`leave` of the host's surfaces (11.3.14): whether the pointer is
    /// over one of the subscriber's surfaces.
    PointerOver { over: bool },
    /// Narrows what this connection receives; the default is everything.
    Subscribe { filter: Filter },
    /// Whether the person's session is locked, sent when it changes and after connecting while
    /// locked. The shell that draws the lock screen says so, because it is the one that knows; a
    /// gesture does nothing on a locked session. The session counts as locked while any connection
    /// says so; a connection that closes stops saying it.
    SessionLock { locked: bool },
    /// The person's own gesture settings (the text of `~/.config/mouse-gestures/gestures.toml`, at most
    /// [`crate::USER_CONFIG_MAX_BYTES`]), sent by the shell after connecting and when the file
    /// changes. The daemon layers it over `/etc/bardo/mouse-gestures/gestures.toml`; empty text takes the
    /// person's layer away. Believed from the shell only.
    UserConfig { toml: String },
}
