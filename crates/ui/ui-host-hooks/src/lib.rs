//! What a quire component calls on the document that hosts it, without linking the renderer that
//! hosts it: `provide_host` (a root's document host), the crash guard for the person's styles
//! (`StylesGuard`), the resolved tokens of an element (`use_resolved`, `use_tokens`), the system
//! clipboard (`clipboard`), the runtime spawner (`TokioSpawner`), and what a window's root may ask
//! of its window and its app (`use_window_control`, `use_app_control`).
//!
//! Dioxus and the design system only: no Blitz, GPU or windowing type is named here. `ui-blitz`
//! (the windows and `launch`), the test harness and `layer-shell-host` are the renderers; they
//! fill the seams this crate defines (`TokenSource`, `Clipboard`, `AppControl`, `WindowControl`,
//! the standalone host) and a UI library depends on this crate instead of on them.

mod app;
pub mod clipboard;
mod focus;
mod host;
mod resolved;
mod spawner;
pub mod styles_guard;
mod window;

pub use app::{AppControl, AppEnded, use_app_control};
pub use focus::FocusFallback;
pub use host::{StandaloneHost, provide_host, set_standalone_host};
pub use resolved::{
    Lookup, Publish, Reading, ResolvedTokens, TokenReader, TokenSource, Tokens, use_resolved,
    use_tokens,
};
pub use spawner::TokioSpawner;
pub use styles_guard::{StylesGuard, use_styles_guard};
pub use window::{WindowControl, use_window_control};
