//! Whether the offered protocols can carry a command.

use crate::backend::{DesktopBackend, backend_for};
use crate::command::{CommandRoute, DesktopCommand};
use crate::protocol::{Protocol, ProtocolSet};

/// What a window command needs on Bardo arm: its model and the ext list that names windows.
const BARDO_WINDOW: &[Protocol] = &[Protocol::BardoDesktop, Protocol::ExtForeignToplevelList];
/// What an app or Space command needs.
const DESKTOP: &[Protocol] = &[Protocol::BardoDesktop];
/// What a shell command needs: bound only by a client that holds the shell role.
const BARDO_SHELL: &[Protocol] = &[Protocol::BardoShell];
/// What the window commands need without Bardo arm.
const WLR_TOPLEVEL: &[Protocol] = &[Protocol::WlrForeignToplevelManagement];

/// The protocols `command` needs on the wlr arm, all of them. For a Bardo-only command, the
/// Bardo protocols it needs. See [`requires_on`] for the arm an offer chooses.
#[must_use]
pub fn requires(command: &DesktopCommand) -> &'static [Protocol] {
    requires_on(Some(DesktopBackend::Wlr), command)
}

/// The protocols `command` needs when the offer chose `backend` (see
/// [`backend_for`]), all of them.
///
/// The plain window commands (`Activate`, `Close`, `Minimise`, ...) go to
/// `bardo_desktop_window_v1` on Bardo arm and to `zwlr-foreign-toplevel-management` on
/// every other; the minimise target goes to `bardo_shell_v1` on Bardo arm.
///
/// ```
/// use desktop_client::{DesktopBackend, DesktopCommand, Protocol, ToplevelKey, requires_on};
///
/// let close = DesktopCommand::Close(ToplevelKey::new(1));
/// assert_eq!(
///     requires_on(Some(DesktopBackend::Wlr), &close),
///     &[Protocol::WlrForeignToplevelManagement]
/// );
/// assert!(requires_on(Some(DesktopBackend::Bardo), &close).contains(&Protocol::BardoDesktop));
/// ```
#[must_use]
pub fn requires_on(
    backend: Option<DesktopBackend>,
    command: &DesktopCommand,
) -> &'static [Protocol] {
    use Protocol::{
        BardoShell, ExtImageCopyCapture, ExtOutputImageCaptureSource, ExtWorkspace,
        WlrOutputManagement,
    };
    let bardo = backend == Some(DesktopBackend::Bardo);
    let by_arm = |on_bardo: &'static [Protocol]| if bardo { on_bardo } else { WLR_TOPLEVEL };
    match command.route() {
        CommandRoute::Toplevel => by_arm(BARDO_WINDOW),
        CommandRoute::MinimiseTarget => by_arm(BARDO_SHELL),
        CommandRoute::Workspace => &[ExtWorkspace],
        CommandRoute::Output => &[WlrOutputManagement],
        CommandRoute::Capture => match command {
            DesktopCommand::CaptureOutput(request) if request.space.is_some() => {
                &[ExtImageCopyCapture, BardoShell]
            }
            _ => &[ExtImageCopyCapture, ExtOutputImageCaptureSource],
        },
        CommandRoute::BardoWindow => BARDO_WINDOW,
        CommandRoute::BardoDesktop => DESKTOP,
        CommandRoute::BardoShell => BARDO_SHELL,
    }
}

/// The first protocol `command` needs that `offered` lacks, on the arm `offered` chooses.
#[must_use]
pub fn missing(offered: &ProtocolSet, command: &DesktopCommand) -> Option<Protocol> {
    requires_on(backend_for(offered), command)
        .iter()
        .copied()
        .find(|p| !offered.contains(*p))
}

/// Whether `offered` carries `command`.
///
/// The compositor may still refuse at run time (a workspace that lacks the ability, a closed
/// toplevel); this answers the protocol question. Whether this client bound `bardo_shell_v1` is the caller's choice, not the offer's; see
/// `admit`.
#[must_use]
pub fn supports(offered: &ProtocolSet, command: &DesktopCommand) -> bool {
    missing(offered, command).is_none()
}
