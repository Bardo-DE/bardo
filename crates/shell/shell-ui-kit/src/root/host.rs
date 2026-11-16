//! What a sill process hands shell-host: quire's faces and the pre-paint pass.

use ui_components::style::fonts::FACES;
use shell_services::env::Dirs;

use super::file_roots::file_roots;

/// The pre-paint pass the daemon installs on its host (`HostConfig::prepaint`): quire's
/// device-pixel layout snap, a no-op at whole scales.
pub fn prepaint() -> layer_shell_host::Prepaint {
    layer_shell_host::Prepaint::hook(ui_blitz::snap_to_device)
}

/// quire's bundled faces (Inter, Inter Display, Space Mono), for `HostConfig::fonts`. Without
/// them every document under shell-host falls back to the system face, since none of them is
/// installed on the system.
pub fn faces() -> layer_shell_host::FontFaces {
    layer_shell_host::FontFaces(FACES.iter().map(|face| face.bytes).collect())
}

/// The host configuration every sill process starts from: quire's faces, the pre-paint pass and
/// the folders pictures may be read from ([`file_roots`], under `dirs`). The daemon adds its
/// settings on top; the demos use it as it is.
pub fn host_config(dirs: &Dirs) -> layer_shell_host::HostConfig {
    layer_shell_host::HostConfig {
        fonts: faces(),
        prepaint: prepaint(),
        file_roots: file_roots(dirs),
        ..layer_shell_host::HostConfig::default()
    }
}
