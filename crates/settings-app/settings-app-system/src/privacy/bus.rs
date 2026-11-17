//! The compositor's Privacy module on the bus, as the Privacy page's link.

use super::PrivacyLink;
use super::set::{forget, set};
use crate::modules::Module;
use settings_app_model::Work;
use settings_types::schema::KeyPath;

/// The compositor's module, reached through the client the pane follows it with.
#[derive(Debug, Clone, Copy)]
pub struct OnClient;

impl PrivacyLink for OnClient {
    fn set<'a>(
        &'a self,
        client: &'a Module,
        path: &'a KeyPath,
        to: bool,
    ) -> Work<'a, Result<(), String>> {
        Box::pin(set(client, path, to))
    }

    fn forget<'a>(&'a self, client: &'a Module, path: &'a KeyPath) -> Work<'a, Result<(), String>> {
        Box::pin(forget(client, path))
    }
}
