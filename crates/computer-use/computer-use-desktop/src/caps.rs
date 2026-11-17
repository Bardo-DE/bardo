//! What a backend can do. The values for the backends are a table (`BackendCaps::of`) so
//! `mode_support` and the tests ask the table rather than a copy of it.

use crate::lease::SeatIsolation;
use serde::{Deserialize, Serialize};

/// Which compositor the computer-use daemon is talking to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendKind {
    /// Stock `KWin`: the nested-session host and the portability reference.
    KwinStock,
    /// The compositor, our own compositor. Until its agent seat lands (the compositor P4) input is a
    /// lease-scoped socket on the person's seat (`Compositor1.LeaseWindow`) and the computer-use daemon masks.
    Compositor,
}

/// Who masks a frame before pixels reach the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaskWhere {
    /// The compositor (fork): the buffer arrives masked.
    Compositor,
    /// The computer-use daemon (stock): `computer_use_capture::apply_masks` runs on the unmasked frame.
    ComputerUseDaemon,
}

/// Who guarantees that a confirmation cannot be answered by the agent's own input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsentGuard {
    /// The compositor accepts physical input only on a consent surface (fork).
    CompositorOrigin,
    /// The computer-use daemon stops emulating while the confirmation shows (stock). A third-party prompt
    /// overlapping the window remains a stated residual risk.
    ComputerUseDaemonSuspendsInput,
}

/// What one backend can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BackendCaps {
    /// Which backend.
    pub kind: BackendKind,
    /// Whether the agent has a seat of its own.
    pub seat: SeatIsolation,
    /// Who masks.
    pub masking: MaskWhere,
    /// Who guards confirmations.
    pub consent: ConsentGuard,
    /// Whether a window on another workspace, or minimized, keeps rendering for capture: frames
    /// keep arriving and the content keeps updating (measured on a stock compositor). When not,
    /// The compositor stops rendering it.
    pub hidden_renders: bool,
}

impl BackendCaps {
    /// The capabilities of `kind`. `KWin`'s row is the portability reference (shared seat,
    /// unmasked capture, hidden windows render) and is unverified until Spike B.
    #[must_use]
    pub fn of(kind: BackendKind) -> BackendCaps {
        match kind {
            BackendKind::KwinStock => BackendCaps {
                kind,
                seat: SeatIsolation::SharedSeat,
                masking: MaskWhere::ComputerUseDaemon,
                consent: ConsentGuard::ComputerUseDaemonSuspendsInput,
                hidden_renders: true,
            },
            // Input reaches only the leased window, but it is still the person's seat and
            // cursor; the compositor does not mask yet (the computer-use daemon does) and does not render a window of
            // another Space, so only in-place runs work.
            BackendKind::Compositor => BackendCaps {
                kind,
                seat: SeatIsolation::SharedSeat,
                masking: MaskWhere::ComputerUseDaemon,
                consent: ConsentGuard::ComputerUseDaemonSuspendsInput,
                hidden_renders: false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kwin_backend_is_a_shared_seat_masked_by_computer_use_daemon_and_guarded_by_suspending_input()
     {
        let kind = BackendKind::KwinStock;
        let caps = BackendCaps::of(kind);
        assert_eq!(caps.kind, kind);
        assert_eq!(caps.seat, SeatIsolation::SharedSeat);
        assert_eq!(caps.masking, MaskWhere::ComputerUseDaemon);
        assert_eq!(caps.consent, ConsentGuard::ComputerUseDaemonSuspendsInput);
        assert!(caps.hidden_renders);
    }

    #[test]
    fn the_compositor_backend_is_a_shared_seat_that_withholds_hidden_windows() {
        let caps = BackendCaps::of(BackendKind::Compositor);
        assert_eq!(caps.kind, BackendKind::Compositor);
        assert_eq!(caps.seat, SeatIsolation::SharedSeat);
        assert_eq!(caps.masking, MaskWhere::ComputerUseDaemon);
        assert_eq!(caps.consent, ConsentGuard::ComputerUseDaemonSuspendsInput);
        assert!(!caps.hidden_renders);
    }
}
