//! The look's settings I/O: `appearance.toml` as a settings document, the settings portal's
//! preferences, the icon-theme lookup, the Spaces and style files, and (with `dioxus`) the one
//! signal a surface resolves its look from. Any settings file goes through
//! `settings-store-fs`; this crate is what is specific to the toolkit's look.
//!
//! `ds` stays effect-free; everything here touches the disk or the bus. A watch or the portal
//! runs as a task on the [`base_spawn::Spawner`] it is handed: no runtime is named here. Feature `dioxus`
//! adds [`use_environment`], the one signal a surface resolves its look from.

mod appearance;
#[cfg(feature = "dioxus")]
mod environment;
pub mod icon_assets;
mod portal;
pub mod published_parts;
mod spaces_storage;

pub use appearance::{AppearanceFile, AppearanceSettings, IconStyle, IconsSettings, bardo_schema};
#[cfg(feature = "dioxus")]
pub use environment::{Environment, use_environment};
pub use icon_assets::{app_icon_path, apps_dir};
pub use portal::{
    SystemPrefsSource, SystemPrefsWatch, contrast_from_portal, reduced_motion_from_portal,
    scheme_from_portal,
};
pub use published_parts::{PublishedParts, discover_parts, parts_dirs, parts_dirs_from};
pub use spaces_storage::{Fixup, Origin, SpacesLoad, SpacesStorage};
pub use ui_style::kit::UserStyle;
