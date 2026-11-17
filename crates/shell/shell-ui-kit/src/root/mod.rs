//! What every surface's document is rooted in: the seat's signals bridge and [`SurfaceRoot`].
//!
//! Every root gets quire's document host through `ui_host_hooks::provide_host()` and the keymap through
//! [`provide_keymap`], called at the top of the root component, and quire's `HostSignals` through
//! [`use_host_signals`], called by `mount`, `mount_lock` and `PopupFrame`.

mod click_away;
mod keymap;
mod root_style;
mod signals;
mod styles_notice;
mod surface_root;

pub use click_away::ClickAway;
pub use keymap::{desktop_keys, provide_keymap};
pub use root_style::{RootStyle, SurfaceName, use_root_style};
pub use signals::{use_host_scale, use_host_signals};
pub use styles_notice::StylesNotice;
pub use surface_root::SurfaceRoot;
