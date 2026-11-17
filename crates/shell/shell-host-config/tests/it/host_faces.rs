//! Every bardo-shell process registers quire's faces with layer-shell-host: a document under the host that
//! names Inter must find Inter, not the system face.

use std::path::Path;

use layer_shell_host::registered_families;
use shell_model::dirs::Dirs;

#[test]
fn the_host_config_registers_quires_faces() {
    let families = registered_families(&shell_host_config::host_config(&Dirs::under(Path::new(
        "/scratch",
    ))));
    for family in ["Inter", "Inter Display", "Space Mono"] {
        assert!(
            families.iter().any(|known| known == family),
            "{family} is registered (got {families:?})"
        );
    }
}
