//! accountd, hosted in the test process over fakes and reached as syncd reaches it: through
//! account-client's in-process transport, as syncd (`org.bardo.Sync`). What the real accountd is
//! on a bus, this is in memory: the registry, the secrets and the audit are memory, the sheet
//! host is a person who allows the first account of every consent ask (Always) and whose
//! conversations (a sign-in) are scripted, and the relays reach the fake servers. No bus for
//! accounts, no real keyring.
#![allow(
    dead_code,
    reason = "a shared test helper: each test binary uses only part of it"
)]

use account_client::{AccountClient, Grants};
use account_client_inproc::{AccountRelays, Hosted as HostedLink};
use account_core::consent::{ConsentAnswer, ConsentAsk, GrantScope};
use account_fake::{FakeProvider, MemoryStore, RecordingAudit, ScriptedLink, ScriptedSheets};
use account_secrets::MemorySecrets;
use account_service::AccountService;
use account_service_core::registry::Registry;
use account_sheet::host::{SheetError, SheetOpen, Sheets};
use account_wire::ParentWindow;
use base_time::FixedClock;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use sync_daemon::keep::{Keep, sync_app};
use sync_driver::hub::Hub;
use sync_xdg::Paths;

/// The person at accountd's sheet host: every consent ask is allowed, for the first account it
/// offers, for good, and the asks are counted; the conversations (a sign-in) are `inner`'s.
#[derive(Debug)]
pub struct Person {
    inner: ScriptedSheets,
    asked: Arc<AtomicUsize>,
}

impl Sheets for Person {
    type Link = ScriptedLink;

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn consent(&self, ask: ConsentAsk, _window: &ParentWindow) -> ConsentAnswer {
        self.asked.fetch_add(1, Ordering::SeqCst);
        match ask.accounts.first() {
            Some(choice) => ConsentAnswer::Allow {
                account: choice.account.clone(),
                scope: GrantScope::Always,
            },
            None => ConsentAnswer::Dismissed,
        }
    }

    fn conversation(
        &self,
        open: SheetOpen,
    ) -> impl std::future::Future<Output = Result<Self::Link, SheetError>> + Send {
        self.inner.conversation(open)
    }
}

pub type Service =
    AccountService<FakeProvider, MemorySecrets, Person, FixedClock, MemoryStore, RecordingAudit>;

pub type Link = HostedLink<Service, AccountRelays>;

/// The hosted service and the handles syncd uses, as syncd (`org.bardo.Sync`).
pub struct Hosted {
    pub service: Arc<Service>,
    pub account_client: AccountClient<Link>,
    pub accounts: Arc<Grants<Link>>,
    pub store: MemoryStore,
    pub audit: RecordingAudit,
    /// How many consent sheets the person has been shown.
    pub consents: Arc<AtomicUsize>,
}

/// A service over `provider`, starting from `registry` and `secrets`, and the handles over it.
pub fn host(provider: FakeProvider, registry: Registry, secrets: MemorySecrets) -> Hosted {
    host_with(provider, registry, secrets, ScriptedSheets::default())
}

/// [`host`], with the conversations of the sheet host as `sheets` scripts them.
pub fn host_with(
    provider: FakeProvider,
    registry: Registry,
    secrets: MemorySecrets,
    sheets: ScriptedSheets,
) -> Hosted {
    let store = MemoryStore::default();
    let audit = RecordingAudit::default();
    let consents = Arc::new(AtomicUsize::new(0));
    let person = Person {
        inner: sheets,
        asked: Arc::clone(&consents),
    };
    let service = Arc::new(
        AccountService::new(
            vec![provider],
            registry,
            secrets,
            person,
            FixedClock(account_fake::NOW),
        )
        .with_store(store.clone())
        .with_audit(audit.clone()),
    );
    let link = HostedLink::new(Arc::clone(&service), sync_app()).with_relays(AccountRelays::new());
    let account_client = AccountClient::over(link);
    Hosted {
        accounts: Arc::new(account_client.grants()),
        account_client,
        service,
        store,
        audit,
        consents,
    }
}

/// What syncd keeps of the accounts, over `hosted`; a removal wipes what `paths` and `hub` name.
pub fn keep(hosted: &Hosted, paths: Paths, hub: Hub) -> Keep<Link> {
    Keep::new(&hosted.account_client, paths, hub)
}
