//! `provide_host`, the one call that gives a root quire's document host, and the seam the renderer
//! fills for a root that has none.
//!
//! A window, the test harness and a layer-shell surface's root all read the host the same way. A
//! window's and the harness's document already carries one (the renderer provides it as a root
//! context before the app renders). A shell surface's root does not: its document is built by a
//! host that names no design system, so the program that links the renderer registers, once, how
//! to build a host for such a root ([`set_standalone_host`]; `ui_blitz::use_standalone_host`).

use std::rc::Rc;
use std::sync::OnceLock;

use dioxus::prelude::*;
use ui_components::prelude::DocumentHost;

/// How the renderer builds the host of a root that has none: it also provides the host's
/// contexts (the clipboard, the token source) to the root, as a window does.
pub type StandaloneHost = fn() -> Rc<dyn DocumentHost>;

static STANDALONE: OnceLock<StandaloneHost> = OnceLock::new();

/// Register how a root with no host gets one. The first registration stands: a program registers
/// once at start, and a second call (a test that runs in the same process) changes nothing.
pub fn set_standalone_host(build: StandaloneHost) {
    let _first_wins = STANDALONE.set(build);
}

/// Give this root quire's document host, once, before any quire component reads the document.
///
/// A root that already has a host (a window's, the harness's) keeps it, so a component calling this
/// under one takes what the window wired. A shell surface's root has none: it is built by the
/// registered standalone host, which has the window's click-focus part (a press on nothing
/// focusable leaves the keyboard on the nearest focusable ancestor, so Escape still reaches the
/// surface), no lookup by selector, no keeper for removed elements, and edit surfaces fed no IME
/// events: the window loop that owns those is `launch`'s and the harness's. Nor does it reach a
/// window shell, so [`clipboard::write_text`](crate::clipboard::write_text) answers
/// [`ClipboardError::NoShell`](crate::clipboard::ClipboardError::NoShell) there, never a success
/// that wrote nothing.
///
/// # Panics
///
/// When the root has no host and no program registered one with [`set_standalone_host`]: a
/// caller contract, like a missing context.
#[must_use]
pub fn provide_host() -> Rc<dyn DocumentHost> {
    use_hook(|| {
        try_consume_context::<Rc<dyn DocumentHost>>().unwrap_or_else(|| {
            let build = STANDALONE.get().expect(
                "the root has no document host and none was registered with `set_standalone_host`",
            );
            build()
        })
    })
}
