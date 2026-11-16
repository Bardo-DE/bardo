//! Sending a Quire command.

use wayland_client::Proxy;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::wl_surface::WlSurface;

use desktop_protocols::client::bardo_desktop_v1::{
    bardo_desktop_app_v1::BardoDesktopAppV1, bardo_desktop_manager_v1::BardoDesktopManagerV1,
    bardo_desktop_space_v1::BardoDesktopSpaceV1, bardo_desktop_window_v1::BardoDesktopWindowV1,
};
use desktop_protocols::client::bardo_shell_v1::bardo_shell_v1::BardoShellV1;
use desktop_protocols::vocab::{PlaceTarget, Toggle, WireCode};

use super::wire::Wire;
use crate::command::{CommandRoute, DesktopCommand, SpaceTarget};
use crate::error::CommandError;
use crate::keys::{AppKey, SpaceKey, SurfaceKey, ToplevelKey};
use crate::names::OutputName;
use crate::protocol::Protocol;

/// Whether `command` goes to the Quire objects: the Quire-only commands always, the plain window
/// commands and the minimise target when the offer chose the Quire arm.
pub(super) fn carries(command: &DesktopCommand, quire_arm: bool) -> bool {
    match command.route() {
        CommandRoute::Toplevel | CommandRoute::MinimiseTarget => quire_arm,
        CommandRoute::QuireWindow | CommandRoute::QuireDesktop | CommandRoute::QuireShell => true,
        CommandRoute::Workspace | CommandRoute::Output | CommandRoute::Capture => false,
    }
}

/// The generated enum with the vocab value's code (the two are tested to agree).
fn generated<V: WireCode, G: TryFrom<u32>>(value: V) -> Result<G, CommandError> {
    // The generated enums' `TryFrom<u32>` has nothing to say beyond the code `Code` carries.
    let Ok(generated) = G::try_from(value.code()) else {
        return Err(CommandError::Code(value.code()));
    };
    Ok(generated)
}

fn window(wire: &Wire, key: ToplevelKey) -> Result<&BardoDesktopWindowV1, CommandError> {
    wire.linker
        .object_of(key)
        .and_then(|object| wire.handles.windows.get(&object))
        .ok_or(CommandError::UnknownToplevel(key))
}

fn app(wire: &Wire, key: AppKey) -> Result<&BardoDesktopAppV1, CommandError> {
    wire.handles
        .apps
        .get(&key)
        .ok_or(CommandError::UnknownApp(key))
}

fn space(wire: &Wire, key: SpaceKey) -> Result<&BardoDesktopSpaceV1, CommandError> {
    wire.handles
        .spaces
        .get(&key)
        .ok_or(CommandError::UnknownSpace(key))
}

fn manager(wire: &Wire, route: CommandRoute) -> Result<&BardoDesktopManagerV1, CommandError> {
    wire.handles
        .quire_manager
        .as_ref()
        .ok_or(CommandError::Unsupported {
            route,
            needs: Protocol::QuireDesktop,
        })
}

fn shell(wire: &Wire, route: CommandRoute) -> Result<&BardoShellV1, CommandError> {
    wire.handles
        .quire_shell
        .as_ref()
        .ok_or(CommandError::NotBound {
            route,
            needs: Protocol::QuireShell,
        })
}

fn seat(wire: &Wire) -> Result<&WlSeat, CommandError> {
    wire.handles.seat.as_ref().ok_or(CommandError::NoSeat)
}

fn surface(wire: &Wire, key: SurfaceKey) -> Result<&WlSurface, CommandError> {
    wire.handles
        .surfaces
        .get(&key)
        .ok_or(CommandError::UnknownSurface(key))
}

fn output<'w>(wire: &'w Wire, name: &OutputName) -> Result<&'w WlOutput, CommandError> {
    wire.handles
        .outputs
        .get(name)
        .ok_or_else(|| CommandError::UnknownOutput(name.clone()))
}

/// The identifier the shell protocol names a window by.
fn identifier(wire: &Wire, key: ToplevelKey) -> Result<String, CommandError> {
    let toplevel = wire
        .state
        .toplevels
        .get(key)
        .ok_or(CommandError::UnknownToplevel(key))?;
    toplevel
        .identifier
        .as_ref()
        .map(|id| id.as_str().to_owned())
        .ok_or(CommandError::NoIdentifier(key))
}

/// Sends a command that [`carries`] says is the Quire arm's. A key with no object is
/// `CommandError::Unknown…`; the shell commands need `bardo_shell_v1` bound.
///
/// Activation passes serial 0: the Quire arm's client is the shell, which the compositor trusts
/// to activate without an input serial (FINDINGS).
pub(super) fn send(wire: &Wire, command: DesktopCommand) -> Result<(), CommandError> {
    let route = command.route();
    match command {
        DesktopCommand::Activate(key) => window(wire, key)?.activate(seat(wire)?, 0),
        DesktopCommand::Close(key) => window(wire, key)?.close(),
        DesktopCommand::Minimise(key) => window(wire, key)?.minimise(),
        DesktopCommand::Unminimise(key) => window(wire, key)?.unminimise(),
        DesktopCommand::Maximise(key) => window(wire, key)?.place(generated(PlaceTarget::Fill)?),
        DesktopCommand::Unmaximise(key) => {
            window(wire, key)?.place(generated(PlaceTarget::Restore)?);
        }
        DesktopCommand::Fullscreen { toplevel, .. } => {
            window(wire, toplevel)?.full_screen(generated(Toggle::On)?);
        }
        DesktopCommand::Unfullscreen(key) => {
            window(wire, key)?.full_screen(generated(Toggle::Off)?);
        }
        DesktopCommand::Place { toplevel, target } => {
            window(wire, toplevel)?.place(generated(target)?);
        }
        DesktopCommand::MoveToSpace {
            toplevel,
            space: key,
        } => window(wire, toplevel)?.to_space(space(wire, key)?),
        DesktopCommand::AssignSpaces { toplevel, assign } => {
            window(wire, toplevel)?.assign(generated(assign)?);
        }
        DesktopCommand::ActivateSpace(key) => space(wire, key)?.activate(),
        DesktopCommand::RemoveSpace(key) => space(wire, key)?.remove(),
        DesktopCommand::MoveSpace {
            space: key,
            position,
        } => space(wire, key)?.move_to(position),
        DesktopCommand::CreateSpace { output: name } => {
            manager(wire, route)?.create_space(output(wire, &name)?);
        }
        DesktopCommand::ActivateApp(key) => app(wire, key)?.activate(seat(wire)?, 0),
        DesktopCommand::HideApp(key) => app(wire, key)?.hide(),
        DesktopCommand::UnhideApp(key) => app(wire, key)?.unhide(),
        DesktopCommand::HideOtherApps(key) => app(wire, key)?.hide_others(),
        DesktopCommand::QuitApp(key) => app(wire, key)?.quit(),
        DesktopCommand::ForceQuitApp(key) => app(wire, key)?.force_quit(),
        DesktopCommand::ReopenApp(key) => app(wire, key)?.reopen(),
        shell_command => return send_shell(wire, shell_command),
    }
    Ok(())
}

/// The shell object, when it is bound at `version` or later.
fn shell_since(
    wire: &Wire,
    route: CommandRoute,
    version: u32,
) -> Result<&BardoShellV1, CommandError> {
    let shell = shell(wire, route)?;
    if shell.version() < version {
        return Err(CommandError::NotBound {
            route,
            needs: Protocol::QuireShell,
        });
    }
    Ok(shell)
}

/// Replaces the Space targets of the output `name` with `targets`.
fn set_space_targets(
    wire: &Wire,
    route: CommandRoute,
    name: &OutputName,
    targets: Vec<SpaceTarget>,
) -> Result<(), CommandError> {
    let shell = shell_since(wire, route, 2)?;
    let output = output(wire, name)?;
    shell.space_target_clear(output);
    for target in targets {
        let rect = target.rect;
        shell.space_target(
            output,
            target.space.as_str().to_owned(),
            rect.x,
            rect.y,
            rect.w,
            rect.h,
        );
    }
    Ok(())
}

/// The commands that go to `bardo_shell_v1`.
fn send_shell(wire: &Wire, command: DesktopCommand) -> Result<(), CommandError> {
    let route = command.route();
    match command {
        DesktopCommand::SetMinimiseTarget {
            toplevel,
            surface: key,
            rect,
        } => shell(wire, route)?.set_minimise_target(
            surface(wire, key)?,
            identifier(wire, toplevel)?,
            rect.x,
            rect.y,
            rect.w,
            rect.h,
        ),
        DesktopCommand::ClearMinimiseTarget(key) => {
            shell(wire, route)?.clear_minimise_target(identifier(wire, key)?);
        }
        DesktopCommand::SetShellRole { surface: key, role } => {
            shell(wire, route)?.set_role(surface(wire, key)?, generated(role)?);
        }
        DesktopCommand::PickSwitcher(index) => shell(wire, route)?.switcher_pick(index),
        DesktopCommand::CancelSwitcher => shell(wire, route)?.switcher_cancel(),
        DesktopCommand::Overview { mode, toggle } => {
            shell(wire, route)?.overview(generated(mode)?, generated(toggle)?);
        }
        DesktopCommand::ShowDesktop(toggle) => shell(wire, route)?.show_desktop(generated(toggle)?),
        DesktopCommand::PickWindowMenu { window, entry } => {
            shell(wire, route)?.window_menu_pick(window.as_str().to_owned(), generated(entry)?);
        }
        DesktopCommand::PickSplit(window) => {
            shell(wire, route)?.split_pick(window.as_str().to_owned());
        }
        DesktopCommand::CancelSplit => shell(wire, route)?.split_cancel(),
        DesktopCommand::DivideSplit { window, share } => {
            shell(wire, route)?.split_divide(window.as_str().to_owned(), share);
        }
        DesktopCommand::SetSpaceTargets {
            output: name,
            targets,
        } => set_space_targets(wire, route, &name, targets)?,
        DesktopCommand::SetNewSpaceTarget { output: name, rect } => {
            let shell = shell_since(wire, route, 3)?;
            let output = output(wire, &name)?;
            // An empty rectangle removes the target.
            let rect = rect.unwrap_or_default();
            shell.space_target_new(output, rect.x, rect.y, rect.w, rect.h);
        }
        DesktopCommand::AnswerPermission { ask, answer } => {
            shell_since(wire, route, 5)?.permission_answer(ask.get(), generated(answer)?);
        }
        DesktopCommand::AnswerTrial { trial, answer } => {
            shell_since(wire, route, 6)?.displays_trial_answer(trial.get(), generated(answer)?);
        }
        DesktopCommand::SetFullScreenBar(mode) => {
            shell_since(wire, route, 7)?.set_full_screen_bar(generated(mode)?);
        }
        DesktopCommand::SetMotion(motion) => {
            shell_since(wire, route, 7)?.set_motion(generated(motion)?);
        }
        // The window, Space and app commands are `send`'s; the others are another family's
        // (`carries` keeps these out of here).
        DesktopCommand::Activate(_)
        | DesktopCommand::Close(_)
        | DesktopCommand::Minimise(_)
        | DesktopCommand::Unminimise(_)
        | DesktopCommand::Maximise(_)
        | DesktopCommand::Unmaximise(_)
        | DesktopCommand::Fullscreen { .. }
        | DesktopCommand::Unfullscreen(_)
        | DesktopCommand::Place { .. }
        | DesktopCommand::MoveToSpace { .. }
        | DesktopCommand::AssignSpaces { .. }
        | DesktopCommand::ActivateSpace(_)
        | DesktopCommand::RemoveSpace(_)
        | DesktopCommand::MoveSpace { .. }
        | DesktopCommand::CreateSpace { .. }
        | DesktopCommand::ActivateApp(_)
        | DesktopCommand::HideApp(_)
        | DesktopCommand::UnhideApp(_)
        | DesktopCommand::HideOtherApps(_)
        | DesktopCommand::QuitApp(_)
        | DesktopCommand::ForceQuitApp(_)
        | DesktopCommand::ReopenApp(_)
        | DesktopCommand::ActivateWorkspace(_)
        | DesktopCommand::DeactivateWorkspace(_)
        | DesktopCommand::AssignWorkspace { .. }
        | DesktopCommand::RemoveWorkspace(_)
        | DesktopCommand::CreateWorkspace { .. }
        | DesktopCommand::CommitWorkspaces
        | DesktopCommand::ApplyOutputs(_)
        | DesktopCommand::TestOutputs(_)
        | DesktopCommand::CaptureOutput(_) => {}
    }
    Ok(())
}
