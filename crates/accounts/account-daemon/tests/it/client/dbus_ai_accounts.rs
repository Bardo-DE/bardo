//! `AiAccounts` (the AI broker's calls) against the real accountd front end on a private bus: the
//! permits an app holds read back as typed verdicts, a key arriving on a descriptor, once for an
//! allow-once permit, the changes a listener is told of, and the refusals of every caller that is
//! not the broker. The keys are fakes; nothing real is touched.

use super::common;

use account_callers::{Caller, CallerRole};
use account_capability::{CapabilityKind, StorageScope};
use account_client::{AiAccountsError, LinkError};
use account_client_dbus::Bus;
use account_core::consent::{Decision, Grant, GrantScope, Usage, Verdict};
use account_core::{AccountId, AppId, AppName, Credential, DataClass, GrantId, GrantKey};
use account_core::{Isolation, SecretKey, SecretPurpose, SecretText, SpaceScope};
use account_daemon::{Options, TableCallers, serve_with};
use account_dbus::BusStream;
use account_fake::{MemoryStore, NOW, ScriptedSheets, llm_account, llm_provider};
use account_ids::UnixSeconds;
use account_ids::clock::FixedClock;
use account_secrets::{MemorySecrets, Secrets};
use account_service::AccountService;
use account_service_core::registry::Registry;
use account_wire::Refusal;
use common::bus::PrivateBus;
use std::sync::Arc;

const KEY: &str = "sk-fake-client-0001";

fn notes() -> AppId {
    AppId {
        name: AppName::parse("org.example.Notes").expect("name"),
        isolation: Isolation::Flatpak,
    }
}

fn permit(id: &str, scope: GrantScope) -> Grant {
    Grant {
        id: GrantId::parse(id).expect("id"),
        key: GrantKey {
            app: notes(),
            account: llm_account().id,
            kind: CapabilityKind::Llm,
            class: DataClass::Prompt,
            usage: Usage::Interactive,
            space: SpaceScope::Any,
            reach: StorageScope::AppFolder,
        },
        decision: Decision::Allow,
        scope,
        at: UnixSeconds(1),
    }
}

type Service = AccountService<
    account_fake::FakeProvider,
    MemorySecrets,
    ScriptedSheets,
    FixedClock,
    MemoryStore,
>;

struct World {
    bus: PrivateBus,
    callers: Arc<TableCallers>,
    service: Arc<Service>,
    _connection: zbus::Connection,
}

impl World {
    async fn start() -> Self {
        let bus = PrivateBus::start();
        let callers = Arc::new(TableCallers::new());
        let connection = bus.connect().await;
        let secrets = MemorySecrets::default();
        secrets
            .put(
                &SecretKey {
                    account: llm_account().id,
                    purpose: SecretPurpose::ApiKey,
                },
                &Credential::ApiKey(SecretText::new(KEY)),
            )
            .await
            .expect("key");
        let service: Arc<Service> = Arc::new(
            AccountService::new(
                vec![llm_provider()],
                Registry {
                    accounts: vec![llm_account()],
                    grants: vec![
                        permit("g-always", GrantScope::Always),
                        permit("g-once", GrantScope::Once),
                    ],
                    toggles: vec![],
                },
                secrets,
                ScriptedSheets::answering([]),
                FixedClock(NOW),
            )
            .with_store(MemoryStore::default()),
        );
        serve_with(
            &connection,
            Arc::clone(&service),
            Arc::clone(&callers),
            Options::default(),
        )
        .await
        .expect("accountd serves");
        Self {
            bus,
            callers,
            service,
            _connection: connection,
        }
    }

    async fn client(&self, role: CallerRole) -> zbus::Connection {
        let connection = self.bus.connect().await;
        let name = connection.unique_name().expect("name").to_string();
        self.callers.introduce_as(
            &name,
            Caller {
                app: AppId {
                    name: AppName::parse("org.bardo.Inference").expect("name"),
                    isolation: Isolation::Unsandboxed,
                },
                role,
            },
        );
        connection
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_broker_reads_verdicts_accounts_and_a_key_once_where_the_permit_is_once() {
    let world = World::start().await;
    let accounts = Bus::over(world.client(CallerRole::AiBroker).await).ai_accounts();
    let verdicts = accounts
        .verdicts(&notes(), DataClass::Prompt, Usage::Interactive)
        .await
        .expect("verdicts");
    assert_eq!(verdicts.len(), 1);
    assert_eq!(verdicts[0].account, llm_account().id);
    assert!(
        matches!(verdicts[0].verdict, Verdict::Granted { .. }),
        "{:?}",
        verdicts[0]
    );
    let none = accounts
        .verdicts(&notes(), DataClass::Mail, Usage::Interactive)
        .await
        .expect("verdicts");
    assert_eq!(none[0].verdict, Verdict::Ask);

    let key = accounts
        .key(&GrantId::parse("g-always").expect("id"))
        .await
        .expect("the key");
    assert_eq!(key.expose(), KEY);
    assert!(!format!("{key:?}").contains("sk-fake"));
    let once = GrantId::parse("g-once").expect("id");
    assert_eq!(accounts.key(&once).await.expect("first").expose(), KEY);
    assert_eq!(
        accounts.key(&once).await.map(drop),
        Err(AiAccountsError::Refused(Refusal::UnknownGrant)),
        "an allow-once permit is spent by a read"
    );
    let listed = accounts.accounts().await.expect("accounts");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].account, llm_account().id);
    assert!(accounts.servers().await.expect("servers").is_empty());
    assert_eq!(
        accounts
            .server_key(&AccountId::parse("nobody").expect("id"))
            .await
            .map(drop),
        Err(AiAccountsError::Refused(Refusal::UnknownGrant))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_change_to_the_registry_reaches_the_broker_as_a_change() {
    let world = World::start().await;
    let accounts = Bus::over(world.client(CallerRole::AiBroker).await).ai_accounts();
    let mut changes = accounts.changes().await.expect("changes");
    // The broker has called, so accountd knows it.
    accounts.accounts().await.expect("accounts");
    world
        .service
        .revoke_grant(&GrantId::parse("g-always").expect("id"))
        .await
        .expect("revoked");
    tokio::time::timeout(
        base_testkit::GENEROUS,
        std::future::poll_fn(|cx| std::pin::Pin::new(&mut changes).poll_next(cx)),
    )
    .await
    .expect("told in time")
    .expect("open")
    .expect("well formed");
}

#[tokio::test(flavor = "multi_thread")]
async fn no_other_caller_is_served() {
    let world = World::start().await;
    for role in [
        CallerRole::App,
        CallerRole::PorterDaemon,
        CallerRole::Agent,
        CallerRole::AgentLauncher,
    ] {
        let accounts = Bus::over(world.client(role).await).ai_accounts();
        assert!(
            matches!(
                accounts.accounts().await,
                Err(AiAccountsError::Link(LinkError::Denied))
            ),
            "{role:?}"
        );
    }
}
