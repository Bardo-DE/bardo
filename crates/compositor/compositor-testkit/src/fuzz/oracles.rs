//! The five oracles of 11 section 3.5, checked after every op:
//!
//! 1. The compositor is alive (a pump that panics is a failure);
//! 2. a client that got a protocol error got a documented one;
//! 3. a healthy bystander completes a roundtrip and its state is unchanged;
//! 4. `Model::check_invariants` holds;
//! 5. privileged globals never appear on a connection that may not see them (I7).

use super::failure::{FuzzError, PanicSite, panic_message};
use super::host::FuzzHost;
use crate::fixture::ClientId;
use compositor_core::policy::{ConnectionRole, GlobalKind, advertised};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// What a bystander looks like when nothing has touched it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    globals: Vec<(String, u32)>,
    objects: Vec<(u32, String)>,
}

/// The bystander's current look.
pub(super) fn snapshot<H: FuzzHost>(host: &mut H, id: ClientId) -> Snapshot {
    let client = host.client(id);
    Snapshot {
        globals: client
            .globals()
            .contents()
            .clone_list()
            .into_iter()
            .map(|global| (global.interface, global.version))
            .collect(),
        objects: client.live_objects(),
    }
}

/// Oracle 1: pumps, and says so if the compositor panicked doing it.
pub(super) fn alive_pump<H: FuzzHost>(host: &mut H) -> Result<(), FuzzError> {
    catch_unwind(AssertUnwindSafe(|| host.pump()))
        .map(|_progressed| ())
        .map_err(|payload| FuzzError::Panicked(PanicSite::Pump, panic_message(&*payload)))
}

/// Oracle 2 for one client: no protocol error, or a documented one.
pub(super) fn documented_error<H: FuzzHost>(host: &mut H, id: ClientId) -> Result<(), FuzzError> {
    let client = host.client(id);
    match (client.protocol_error(), client.error_is_documented()) {
        (Some(error), Some(false)) => Err(FuzzError::UndocumentedError { client: id, error }),
        _ => Ok(()),
    }
}

/// Oracle 3: the bystander answers a sync and looks as it did.
pub(super) fn bystander_ok<H: FuzzHost>(
    host: &mut H,
    id: ClientId,
    before: &Snapshot,
) -> Result<(), FuzzError> {
    let done = host.client(id).sync()?;
    alive_pump(host)?;
    if let Some(error) = host.client(id).protocol_error() {
        return Err(FuzzError::BystanderError(error));
    }
    if !done.is_done() {
        return Err(FuzzError::BystanderStalled);
    }
    let after = snapshot(host, id);
    if &after != before {
        return Err(FuzzError::BystanderChanged {
            before: Box::new(before.clone()),
            after: Box::new(after),
        });
    }
    Ok(())
}

/// Oracle 4.
pub(super) fn invariants<H: FuzzHost>(host: &H) -> Result<(), FuzzError> {
    host.invariants().map_err(FuzzError::InvariantBroke)
}

/// The privileged globals no `Public` or `Sandboxed` connection may be offered.
const PRIVILEGED: [&str; 4] = [
    "bardo_shell_v1",
    "bardo_desktop_manager_v1",
    "bardo_agent_manager_v1",
    "bardo_trusted_manager_v1",
];

/// Oracle 5 for one client of `role`.
pub(super) fn privilege<H: FuzzHost>(
    host: &mut H,
    id: ClientId,
    role: &ConnectionRole,
) -> Result<(), FuzzError> {
    let seen: Vec<String> = host
        .client(id)
        .globals()
        .contents()
        .clone_list()
        .into_iter()
        .map(|global| global.interface)
        .collect();
    let privileged_role = matches!(role, ConnectionRole::Shell | ConnectionRole::Agent);
    for interface in &seen {
        if !privileged_role && PRIVILEGED.contains(&interface.as_str()) {
            return Err(FuzzError::PrivilegedOffered {
                role: role.clone(),
                interface: interface.clone(),
            });
        }
        if let Some(kind) = kind_of(interface)
            && !advertised(kind, role)
        {
            return Err(FuzzError::GlobalNotForRole {
                role: role.clone(),
                interface: interface.clone(),
            });
        }
    }
    Ok(())
}

/// The P0 global an interface name is (`wl_output` too).
fn kind_of(interface: &str) -> Option<GlobalKind> {
    Some(match interface {
        "wl_compositor" => GlobalKind::Compositor,
        "wl_subcompositor" => GlobalKind::Subcompositor,
        "wl_shm" => GlobalKind::Shm,
        "wl_output" | "zxdg_output_manager_v1" => GlobalKind::Output,
        "wp_viewporter" => GlobalKind::Viewporter,
        "wp_single_pixel_buffer_manager_v1" => GlobalKind::SinglePixel,
        "wp_alpha_modifier_v1" => GlobalKind::AlphaModifier,
        "wp_presentation" => GlobalKind::Presentation,
        "wp_fractional_scale_manager_v1" => GlobalKind::FractionalScale,
        "wp_fifo_manager_v1" => GlobalKind::Fifo,
        "wp_commit_timing_manager_v1" => GlobalKind::CommitTiming,
        "wp_content_type_manager_v1" => GlobalKind::ContentType,
        "wp_color_manager_v1" => GlobalKind::ColorManagement,
        "xdg_wm_base" => GlobalKind::XdgShell,
        "zxdg_decoration_manager_v1" => GlobalKind::XdgDecoration,
        "xdg_activation_v1" => GlobalKind::XdgActivation,
        "xdg_wm_dialog_v1" => GlobalKind::XdgDialog,
        "xdg_toplevel_icon_manager_v1" => GlobalKind::XdgToplevelIcon,
        "xdg_toplevel_tag_manager_v1" => GlobalKind::XdgToplevelTag,
        "zxdg_exporter_v2" | "zxdg_importer_v2" => GlobalKind::XdgForeign,
        "xdg_system_bell_v1" => GlobalKind::SystemBell,
        "bardo_window_manager_v1" => GlobalKind::BardoWindow,
        "zwlr_layer_shell_v1" => GlobalKind::LayerShell,
        "ext_session_lock_manager_v1" => GlobalKind::SessionLock,
        "wl_seat" => GlobalKind::Seat,
        "wp_cursor_shape_manager_v1" => GlobalKind::CursorShape,
        "zwp_pointer_constraints_v1" => GlobalKind::PointerConstraints,
        "zwp_relative_pointer_manager_v1" => GlobalKind::RelativePointer,
        "zwp_pointer_gestures_v1" => GlobalKind::PointerGestures,
        "zwp_keyboard_shortcuts_inhibit_manager_v1" => GlobalKind::ShortcutsInhibit,
        "zwp_tablet_manager_v2" => GlobalKind::Tablet,
        "wl_data_device_manager" => GlobalKind::DataDevice,
        "zwp_primary_selection_device_manager_v1" => GlobalKind::PrimarySelection,
        "ext_data_control_manager_v1" => GlobalKind::ExtDataControl,
        "zwlr_data_control_manager_v1" => GlobalKind::WlrDataControl,
        "zwp_linux_dmabuf_v1" => GlobalKind::Dmabuf,
        "zwp_text_input_manager_v3" => GlobalKind::TextInput,
        "zwp_input_method_manager_v2" => GlobalKind::InputMethod,
        "zwp_virtual_keyboard_manager_v1" => GlobalKind::VirtualKeyboard,
        "ext_idle_notifier_v1" => GlobalKind::IdleNotify,
        "zwp_idle_inhibit_manager_v1" => GlobalKind::IdleInhibit,
        "ext_foreign_toplevel_list_v1" => GlobalKind::ForeignToplevelList,
        "wp_security_context_manager_v1" => GlobalKind::SecurityContext,
        "ext_background_effect_manager_v1" => GlobalKind::BackgroundEffect,
        // Declared at P1-Z; the fill lanes advertise them.
        "zwlr_output_manager_v1" => GlobalKind::OutputManagement,
        "zwlr_output_power_manager_v1" => GlobalKind::OutputPower,
        "zwlr_gamma_control_manager_v1" => GlobalKind::Gamma,
        "zwlr_foreign_toplevel_manager_v1" => GlobalKind::WlrToplevel,
        "ext_workspace_manager_v1" => GlobalKind::Workspace,
        "ext_output_image_capture_source_manager_v1"
        | "ext_image_copy_capture_manager_v1"
        | "zwlr_screencopy_manager_v1" => GlobalKind::ImageCapture,
        // P2-Z: the desktop and shell protocols (the shell and agent connections only) and
        // the toplevel capture source (declared; P2-Q3 advertises it).
        "bardo_desktop_manager_v1" => GlobalKind::BardoDesktop,
        "bardo_shell_v1" => GlobalKind::BardoShell,
        "ext_foreign_toplevel_image_capture_source_manager_v1" => GlobalKind::ToplevelCapture,
        _ => return None,
    })
}
