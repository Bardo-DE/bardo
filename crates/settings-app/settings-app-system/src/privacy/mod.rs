//! The calls the Privacy page makes on the compositor's module: allowing or denying a power for a
//! program, and asking again.

mod bus;
#[cfg(feature = "testing")]
mod fake;
mod set;

pub use bus::OnClient;
#[cfg(feature = "testing")]
pub use fake::{FakePrivacy, PrivacyAsked};

use crate::modules::Module;
use settings_app_model::Work;
use std::fmt::Debug;
use settings_types::schema::KeyPath;

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
