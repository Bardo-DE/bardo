//! The second step of a two-step read, with the files checked on disk.

use super::ready::readiness;
use plugin_core::manifest::{ManifestError, Provides, Raw};
use plugin_core::registry::Candidate;
use plugin_core::{Reparsed, reparse_with};

/// Turns the entries of a scan that knew no vocabulary into the owner's own type `P`, and checks
/// the programs and paths of the owner's entries on disk (missing, not executable, outside the
/// folder).
///
/// [`reparse_with`] says what is kept, what is set aside and what is refused.
///
/// # Errors
///
/// Returns the manifest failure that says what is wrong with the entries the owner owns.
pub fn reparse<P: Provides>(
    candidate: Candidate<Raw>,
) -> Result<Reparsed<P>, ManifestError<P::Capability, P::Fault>> {
    reparse_with(candidate, readiness)
}
