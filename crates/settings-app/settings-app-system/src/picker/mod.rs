//! Setting a row of inferd's settings module for the model picker: the module answers when it has
//! put the row in force.

mod bus;
#[cfg(feature = "testing")]
mod fake;
mod set;

pub use bus::OnClient;
#[cfg(feature = "testing")]
pub use fake::{FakePicker, PickerAsked};

use crate::modules::Module;
use settings_app_model::Work;
use std::fmt::Debug;
use settings_types::schema::KeyPath;

/// What the picker asks of inferd's module, through the client the pane follows it with.
pub trait PickerLink: Debug + Send + Sync {
    /// Sets the row at `path` to `word`; the module's own words when it refuses.
    fn set<'a>(
        &'a self,
        client: &'a Module,
        path: &'a KeyPath,
        word: &'a str,
    ) -> Work<'a, Result<(), String>>;
    /// Sets the pool at `path` to `words`; the module refuses a model it does not know.
    fn set_pool<'a>(
        &'a self,
        client: &'a Module,
        path: &'a KeyPath,
        words: &'a [String],
    ) -> Work<'a, Result<(), String>>;
}
