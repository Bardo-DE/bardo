//! The accounts the modules list, read where a page outside the Accounts pane needs them. The
//! reading is `detent-client`'s; this is its signal.

use crate::live::use_live;
use settings_app_client::Client;
use dioxus::prelude::*;

pub(crate) use settings_app_client::Listed;

/// The accounts the modules list, kept current.
pub(crate) fn use_listed() -> Signal<Listed> {
    use_live(Client::listing).state
}
