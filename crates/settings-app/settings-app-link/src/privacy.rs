//! The calls the Privacy page makes on the compositor's module: allowing or denying a power for a
//! program, and asking again.

use crate::modules::Module;
use settings_app_model::Work;
use settings_types::schema::KeyPath;
use std::fmt::Debug;

/// What the Privacy page asks of the compositor's module, through the client the pane follows it
/// with.
pub trait PrivacyLink: Debug + Send + Sync {
    /// Allows or denies at `path`; the module's own words when it refuses.
    fn set<'a>(
        &'a self,
        client: &'a Module,
        path: &'a KeyPath,
        to: bool,
    ) -> Work<'a, Result<(), String>>;
    /// Asks again at `path`; the module's own words when it refuses.
    fn forget<'a>(&'a self, client: &'a Module, path: &'a KeyPath) -> Work<'a, Result<(), String>>;
}
