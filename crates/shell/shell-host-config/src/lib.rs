//! What a bardo-shell process hands layer-shell-host: the UI kit's faces, the pre-paint pass and the
//! folders pictures may be read from. The one place the shell's UI libraries meet the host and
//! the renderer, so they name neither.

mod file_roots;
mod host;

pub use file_roots::file_roots;
pub use host::{faces, host_config, prepaint};
