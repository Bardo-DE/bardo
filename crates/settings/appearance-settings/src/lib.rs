//! The appearance settings, without a toolkit: the words a person picks (theme, accent, motion,
//! transparency, icon style), the keys of `bardo/appearance.toml` that hold them, and that file's
//! schema.
//!
//! A program that only reads what the person chose (the compositor) depends on this crate and
//! names no UI toolkit. The toolkit (`ui-style`) resolves these words to a look; each word is
//! defined once, here.
//!
//! Every struct is read leniently through the settings store, so a bad value costs only its own
//! key, and the stored words are the ones the enums write.

mod accent;
mod file;
mod icon_style;
mod motion;
mod schema;
mod settings;
mod theme;
mod transparency;

pub use accent::Accent;
pub use icon_style::IconStyle;
pub use motion::Motion;
pub use schema::bardo_schema;
pub use settings::{AppearanceFile, AppearanceSettings, IconsSettings};
pub use theme::Theme;
pub use transparency::Transparency;
