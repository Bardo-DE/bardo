//! A supervisor whose look could not be finished says so, so that its loop looks again in
//! seconds (`next_look`) and not at the next ten-minute rescan: a grant list that does not answer
//! yet on a loaded machine must not leave a granted account unmirrored for ten minutes.

use crate::common::bus::PrivateBus;
use crate::common::hosted::host;
use account_client::{GrantsError, LinkError};
use account_core::Candidate;
use account_fake::cloud_provider;
use account_secrets::MemorySecrets;
use account_service_core::registry::Registry;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use sync_driver::hub::{Access, Hub};
use sync_engine::scheduler::{Network, Settings};
use sync_pim_feed::PimKind;
use sync_pim_mirror::{AccountdUnavailableError, PimConfig, PimGrants, PimSupervisor, Wiring};
use sync_xdg::Paths;
use tokio::sync::watch;

/// Grants that cannot be asked while `down` is set, and are none otherwise.
struct Flaky {
    down: Arc<AtomicBool>,
}

impl PimGrants for Flaky {
    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn granted(&self, _kind: PimKind) -> Result<Vec<Candidate>, AccountdUnavailableError> {
        if self.down.load(Ordering::SeqCst) {
            Err(AccountdUnavailableError(GrantsError::Link(
                LinkError::Unreachable,
            )))
        } else {
            Ok(Vec::new())
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_look_that_could_not_ask_for_grants_says_it_was_not_whole_and_the_next_one_is() {
    let bus = PrivateBus::start();
    let hosted = host(
        cloud_provider(),
        Registry::default(),
        MemorySecrets::default(),
    );
    let root = bus.scratch().join("syncd");
    let paths = Paths {
        journals: root.join("state/bardo/sync"),
        mirrors: root.join("data/bardo/sync/vdir"),
        callers_system: root.join("none"),
        callers_user: root.join("none"),
        cloud: root.join("Cloud"),
        drive_cache: root.join("cache/bardo/sync/drives"),
        settings: root.join("config/bardo/sync/syncd.toml"),
    };
    let (_network_up, network) = watch::channel(Network::Unmetered);
    let wiring = Wiring {
        accounts: hosted.accounts,
        hub: Hub::default(),
        paths,
        settings: Settings::quick(),
        network,
        owners: Access::default(),
        clock: sync_core::Clock::new(base_time::SystemClock),
        nonce: "t1".to_owned(),
        millis: Arc::new(sync_engine::clock::ManualMillis::at(1_700_000_000_000)),
    };
    let down = Arc::new(AtomicBool::new(true));
    let mut supervisor = PimSupervisor::new(
        wiring,
        Flaky {
            down: Arc::clone(&down),
        },
        PimConfig::default(),
    );
    assert!(
        !supervisor.tick().await,
        "the grants could not be asked: the look is not whole"
    );
    down.store(false, Ordering::SeqCst);
    assert!(supervisor.tick().await, "asked and answered: whole");
}
