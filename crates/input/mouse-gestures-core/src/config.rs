//! `mouse-gestures/gestures.toml`: the settings a person chooses, their defaults and layering.
//!
//! Also the schema the Settings app reads. Where the file lives and how it is read and watched is
//! the daemon's.

pub mod file;
pub mod layer;
pub mod lenient;
pub mod schema;
pub mod sections;
pub mod units;

pub use file::{Config, parse, parse_table, to_toml};
pub use layer::layered;
pub use lenient::Warning;
pub use schema::{FILE_NAME as SCHEMA_FILE_NAME, Schema};
pub use sections::{GestureSettings, ScrollSettings};
pub use units::{Count, Fraction, Ms, Percent, Px, Um, Units};
