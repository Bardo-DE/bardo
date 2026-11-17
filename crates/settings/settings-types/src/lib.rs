//! The plain data of settings: where in Settings a link points ([`target`]), the schema a program
//! ships ([`schema`]: keys, pages, kinds, the derive's output), the units a key is written in
//! ([`Ms`], [`Percent`], ...), the region choices and the name of a program's config folder
//! ([`AppName`]).
//!
//! No file, no bus, no clock: a service, a compositor or a shell that never reads a settings file
//! can still hold, parse and print all of it. Reading and writing files is `settings-store` and
//! `settings-store-fs`; the live bus modules are `settings-dbus`.

// `#[derive(SettingsSchema)]`'s generated code always writes `::settings_types::schema::...`, so
// the same derive output resolves whether a consumer crate invokes it or this crate does
// (`region.rs`, `schema/behaviour.rs`). This self-referential alias lets the second case resolve
// too, without a second, crate-relative code path in `settings-types-macros`.
extern crate self as settings_types;

mod app_name;
mod region;
pub mod schema;
pub mod target;
mod units;
mod when;

pub use app_name::{AppName, app_folder};
pub use region::{HourCycle, RegionSettings, ResolvedHours, time_locale};
pub use settings_types_macros::{SettingsRow, SettingsSchema};
pub use units::{Count, Fraction, Mins, Ms, Percent, Px, Scalar, Secs, Units};
pub use when::{UnixSecs, UtcOffset, list_date};
