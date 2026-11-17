//! syncd (`org.bardo.Sync1`), as the shell hears it.
//!
//! When a dataset's replica loses most of its
//! files, syncd holds the removal here and tells the shell, which says so in a notification
//! whose action opens Settings at the account. The decision is Settings' (it alone may confirm
//! the discard); the shell offers no removal. The notification is taken back when the hold ends.
//!
//! The service subscribes to `NeedsConfirmation`, then calls `Sync1.Watch()` once (holds already
//! on arrive before it returns), and calls it again whenever `org.bardo.Sync1` gets a new owner:
//! syncd keeps its roster in memory. Without syncd the state stays empty.

mod backend;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::service::ServiceIo;
use shell_model::sync_hold::SyncHoldService;

pub use backend::SyncHoldLinks;

impl Run for SyncHoldService {
    type Kind = Self;
    type Deps = SyncHoldLinks;

    fn run(
        env: &Env,
        deps: Self::Deps,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let bus = env.buses.session.clone();
        async move { backend::run(bus, deps, io).await }
    }
}
