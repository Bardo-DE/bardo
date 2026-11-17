//! What a bardo-shell process hands layer-shell-host: quire's faces and the pre-paint pass.

use shell_model::dirs::Dirs;
use ui_components::style::fonts::FACES;

use crate::file_roots::file_roots;

/// The pre-paint pass the daemon installs on its host (`HostConfig::prepaint`): quire's
/// device-pixel layout snap, a no-op at whole scales.
pub fn prepaint() -> layer_shell_host::Prepaint {
    layer_shell_host::Prepaint::hook(ui_blitz::snap_to_device)
}

/// quire's bundled faces (Inter, Inter Display, Space Mono), for `HostConfig::fonts`.
///
/// Without them every document under layer-shell-host falls back to the system face, since none of
/// them is installed on the system.
#[must_use]
pub fn faces() -> layer_shell_host::FontFaces {
    layer_shell_host::FontFaces(FACES.iter().map(|face| face.bytes).collect())
}

/// The host configuration every bardo-shell process starts from: quire's faces, the pre-paint pass
/// and the folders pictures may be read from ([`file_roots`], under `dirs`).
///
/// It also registers the document host the surfaces' roots get from `ui_host_hooks::provide_host`
/// (the host's documents are built with none), so a process that starts its host from this has it.
///
/// The daemon adds its settings on top; the demos use it as it is.
#[must_use]
pub fn host_config(dirs: &Dirs) -> layer_shell_host::HostConfig {
    ui_blitz::use_standalone_host();
    layer_shell_host::HostConfig {
        fonts: faces(),
        prepaint: prepaint(),
        file_roots: file_roots(dirs),
        ..layer_shell_host::HostConfig::default()
    }
}
