//! The provider files the accounts service ships (`providers/*.toml`), compiled in: an app that links
//! account-provider (mailo on macOS and Windows, a test with nothing installed) has the same files
//! accountd reads from `/usr/share/bardo/accounts/providers`, from the same the accounts rev, with no copy.

use crate::{ProviderSet, ProviderSpec, parse_provider};

/// Every shipped provider file, by its id (the file stem), in id order.
pub const SHIPPED_FILES: &[(&str, &str)] = include!(concat!(env!("OUT_DIR"), "/shipped_files.rs"));

/// The shipped provider files, parsed. Every file parses (the accounts service's `tests/shipped_files.rs`
/// checks each one and that this list is the whole `providers/` directory).
///
/// # Panics
///
/// If a shipped provider file does not parse, which the crate's own tests rule out.
#[must_use]
pub fn shipped_specs() -> Vec<ProviderSpec> {
    SHIPPED_FILES
        .iter()
        .map(|(id, text)| {
            parse_provider(text).unwrap_or_else(|e| panic!("shipped provider file {id}: {e}"))
        })
        .collect()
}

/// The shipped provider files as a set, with no user files layered over them.
#[must_use]
pub fn shipped() -> ProviderSet {
    ProviderSet::layered(shipped_specs(), Vec::new())
}
