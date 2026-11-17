//! The fake mouse gestures daemon feed: a SEQPACKET socket at `$XDG_RUNTIME_DIR/bardo/mouse-gestures/events.sock`
//! replaying recorded `ToHost` lines (the mouse gestures daemon's `replay_to_host` example, P0-PR0). Bodies at
//! P3-Z (the mouse gestures daemon protocol v2).

/// The fake mouse_gestures.
#[derive(Debug, Default)]
pub struct FakeMouseGestures;
