//! The service's effects: what it reaches outside the process.

mod entry;
mod locales;
mod run;
mod scan;

pub use entry::parse_entry;
pub(crate) use run::run;
pub use scan::scan;
