//! Where a surface's keymap comes from: our desktop's keycap, unless the document already has one.

use dioxus::prelude::{provide_context, try_consume_context, use_hook};
use key_remap_client::KeycapSource;
use keymap::{Desktop, Platform};
use ui_components::keys::KeySource;

/// What every shell document loads its keymap from: the conventions of our desktop, the
/// desktop's own bindings and the machine's and the person's keymap files, kept live by keycap.
pub fn desktop_keys() -> KeySource {
    KeySource::new(
        Platform::Linux {
            desktop: Desktop::Ours,
        },
        Box::new(KeycapSource::standard()),
    )
}

/// Provide [`desktop_keys`] to the `Ds` below, called at the top of every root component beside
/// `ui_blitz::provide_host()`. A document that already has a source (the harness, with its fixed
/// platform) keeps it, so a test reads no real system.
pub fn provide_keymap() -> KeySource {
    use_hook(|| {
        try_consume_context::<KeySource>().unwrap_or_else(|| provide_context(desktop_keys()))
    })
}
