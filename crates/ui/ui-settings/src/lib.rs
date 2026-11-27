//! The look's settings I/O: the settings portal's preferences (read by `settings-portal`), the
//! icon-theme lookup, the Spaces and style files, and (with `dioxus`) the one signal a surface
//! resolves its look from. `appearance.toml` itself is `appearance-settings`. Any settings file goes through
//! `settings-store-fs`; this crate is what is specific to the toolkit's look.
//!
//! `ds` stays effect-free; everything here touches the disk or the bus. A watch or the portal
//! runs as a task on the [`base_spawn::Spawner`] it is handed: no runtime is named here. Feature `dioxus`
//! adds [`use_environment`], the one signal a surface resolves its look from.

#[cfg(feature = "dioxus")]
mod environment;
pub mod icon_assets;
mod portal;
pub mod published_parts;
mod spaces_storage;

#[cfg(feature = "dioxus")]
pub use environment::{Environment, use_environment};
pub use icon_assets::{app_icon_path, apps_dir};
pub use portal::{SystemSettingsSource, SystemSettingsWatch};
pub use published_parts::{PublishedParts, discover_parts, parts_dirs, parts_dirs_from};
pub use spaces_storage::{Fixup, Origin, SpacesLoad, SpacesStorage};
pub use ui_style::kit::UserStyle;
