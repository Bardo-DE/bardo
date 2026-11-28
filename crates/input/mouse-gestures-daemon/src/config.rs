//! Where `gestures.toml` lives and how it is read and watched. The settings themselves (sections,
//! units, layering, the schema) are `mouse_gestures_core::config`.

pub mod load;
pub mod watch;

pub use load::{Loaded, config_dir, first_directory, load};
