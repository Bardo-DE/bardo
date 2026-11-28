//! The host that runs relays for an app hosting the accounts service in process: the app owns the runtime and
//! the network, so it spawns the relay (`account-proxy`'s `relay` over a TLS connector it
//! chooses) and the client only hands it the plan and the relay's end of an in-memory duplex.
//!
//! With the feature `relays` the crate brings that host ready made: [`AccountRelays`], which runs
//! `account_proxy::relay` on the app's tokio runtime with the platform's trusted roots. An app or
//! daemon that hosts its own `AccountService` (syncd) hands it to `Hosted::with_relays`.

use account_core::RelayPlan;
use base_stream::DuplexEnd;

/// Runs the relay for a plan the service checked.
pub trait RelayHost: Send + Sync {
    /// Starts a relay for `plan` that talks to the app through `end`, and returns at once. The
    /// relay ends when either side finishes.
    fn run(&self, plan: RelayPlan, end: DuplexEnd);
}

/// No relays hosted: opening an authenticated stream says nobody is reachable, as a daemon that
/// is not running does.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoRelays;

impl RelayHost for NoRelays {
    fn run(&self, plan: RelayPlan, end: DuplexEnd) {
        let _ = (plan, end);
    }
}

#[cfg(feature = "relays")]
pub use account_relays::AccountRelays;

/// The relay host over `account-proxy` (feature `relays`).
#[cfg(feature = "relays")]
mod account_relays {
    use super::RelayHost;
    use account_core::RelayPlan;
    use account_proxy::{RustlsConnect, relay};
    use base_stream::DuplexEnd;
    use std::sync::Arc;
    use tokio::sync::OnceCell;

    /// A [`RelayHost`] that runs `account_proxy::relay` on the caller's tokio runtime, dialling the
    /// endpoint over TCP and rustls.
    ///
    /// Certificate checks are always on; the trusted roots are the platform's (`webpki-roots` when
    /// it has none), read from disk once, on first use, off the async threads. [`RelayHost::run`]
    /// must be called from inside a tokio runtime, as `Hosted` does.
    #[derive(Debug, Clone, Default)]
    pub struct AccountRelays {
        connector: Arc<OnceCell<RustlsConnect>>,
    }

    impl AccountRelays {
        /// A host that trusts the platform's roots.
        #[must_use]
        pub fn new() -> Self {
            Self::default()
        }

        /// A host that dials with `connector` (a test's scratch CA, for one).
        #[must_use]
        pub fn with_connector(connector: RustlsConnect) -> Self {
            Self {
                connector: Arc::new(OnceCell::new_with(Some(connector))),
            }
        }
    }

    impl RelayHost for AccountRelays {
        fn run(&self, plan: RelayPlan, end: DuplexEnd) {
            let cell = Arc::clone(&self.connector);
            tokio::spawn(async move {
                let connector = cell
                    .get_or_try_init(|| async {
                        tokio::task::spawn_blocking(RustlsConnect::platform).await
                    })
                    .await;
                // A connector that could not be built ends the relay, and dropping `end`
                // finishes the app's stream: the app sees nobody reachable.
                if let Ok(connector) = connector {
                    let _ = relay(plan, end, connector).await;
                }
            });
        }
    }
}
