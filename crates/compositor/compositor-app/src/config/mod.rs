//! The stored settings: `compositor/settings.toml` through ui-settings (lenient read, atomic write,
//! a debounced watch), its schema for detent (`bardo-compositor --write-schema`).
//!
//! And the one conversion into the model's `CompositorSettings` (ARCHITECTURE.md, "Add a settings
//! key").

mod action;
mod convert;
mod doc;
mod schema;
mod sections;

pub use action::ActionWords;
pub use convert::to_core;
pub(crate) use doc::watch;
pub use doc::{CompositorDoc, load, xwayland_of};
pub use schema::{schema, write_schema};
pub use sections::{
    BindingsSection, FirstSight, GesturesSection, GpuSection, KeyboardSection, NightLightSection,
    OutputsSection, PointerSection, SpacesSection, TouchpadSection, WindowsSection,
    XwaylandSection, XwaylandUse,
};

/// Why the settings could not be read or the schema written.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// ui-settings refused.
    #[error("settings: {0}")]
    Settings(String),
    /// The schema could not be written.
    #[error("schema: {0}")]
    Schema(#[from] std::io::Error),
}
