//! Acceptance 1 of design/31 §7.1, in process: a Nextcloud account is added by Login Flow v2
//! against the fake; a second app's candidates for Storage are none until consent is accepted, and
//! then one candidate with the account's endpoints. Around it, the rest of what the service does
//! with a real family: sign in again, close the sheet mid-wait, a registry that cannot be
//! written, a service turned off, add-and-allow.
#![cfg(all(feature = "nextcloud", feature = "generic"))]

use crate::common;

use account_capability::{Access, Delta, StorageScope};
use account_core::ProviderHint;
use account_core::consent::{GrantScope, Usage};
use account_core::{AccountId, ProviderId};
use account_core::{AccountState, AppId, AppName, Candidate, DataClass, Family, Isolation};
use account_fake::{Scripted, ScriptedSheets};
use account_fake_servers::{FakeNextcloud, LoginPolicy, NextcloudHandle, Running, shipped};
use account_families::DynProvider;
use account_family_kit::Pacing;
use account_family_nextcloud::NextcloudProvider;
use http_core::SharedHttp;
use account_match::Need;
use account_match::StorageNeed;
use account_service_accounts::AllowFor;
use account_service_core::audit::AuditEvent;
use account_sheet::{FieldKind, SheetView};
use account_wire::{AccountsReply, AccountsRequest};
use account_wire::{ParentWindow, Refusal};
use common::{Brief, Fakes, Kept, Opens, Person, Service, TestSheets, plain, service};
use std::time::Duration;

fn app(name: &str) -> AppId {
    AppId {
        name: AppName::parse(name).expect("app name"),
        isolation: Isolation::Flatpak,
    }
}

fn storage() -> Need {
    Need::Storage(StorageNeed::new(
        Access::ReadWrite,
        Delta::Poll,
        StorageScope::Full,
        true,
    ))
}

/// What `app` is told it may use for `storage()`.
fn candidates_of(service: &Service, app: &AppId) -> Vec<Candidate> {
    service.candidates(app, &storage(), DataClass::Files, Usage::Interactive)
}

fn provider() -> Box<dyn DynProvider> {
    DynProvider::boxed(
        NextcloudProvider::new(
            shipped::nextcloud(),
            SharedHttp::new(Fakes::loopback()),
            Brief,
        )
        .with_pacing(Pacing {
            first: Duration::from_secs(1),
            step: Duration::from_secs(1),
            limit: Duration::from_secs(100_000),
        }),
    )
}

/// Someone adding a Nextcloud: types its address and, when the page is shown, opens it.
fn adding(server: &str) -> Person {
    Person {
        pick: Some(ProviderId::parse("nextcloud").expect("id")),
        answers: vec![plain(FieldKind::Server, server)],
        opens_page: Some(Opens::Now),
        ..Person::default()
    }
}

async fn nextcloud() -> Running<NextcloudHandle> {
    let running = FakeNextcloud::start("alice").await.expect("nextcloud");
    running.set_login_policy(LoginPolicy::Approve);
    running
}

/// The id the account of "alice" at the fake gets: the provider and the label (login, host and
/// the fake's port) as a slug.
fn expected_id(nextcloud: &Running<NextcloudHandle>, suffix: &str) -> String {
    let port = nextcloud.base_url().rsplit(':').next().expect("port");
    format!("nextcloud-alice-127.0.0.1-{port}{suffix}")
}

fn added(reply: AccountsReply) -> AccountId {
    match reply {
        AccountsReply::Added(id) => id,
        other => panic!("expected Added, got {other:?}"),
    }
}

fn consent(answers: Vec<Scripted>) -> ScriptedSheets {
    ScriptedSheets::answering(answers)
}

async fn with_account(
    nextcloud: &Running<NextcloudHandle>,
    consent: ScriptedSheets,
    people: Vec<Person>,
) -> (Service, Kept, AccountId) {
    let (service, kept) = service(vec![provider()], TestSheets::new(people, consent));
    let id = added(
        service
            .handle(
                &app("org.bardo.Photos"),
                AccountsRequest::AddAccount {
                    hint: ProviderHint::Any,
                    window: ParentWindow::Unparented,
                },
            )
            .await,
    );
    let _ = nextcloud;
    (service, kept, id)
}

#[tokio::test]
async fn adding_a_nextcloud_account_by_login_flow_then_a_second_app_asks_and_is_granted() {
    let nextcloud = nextcloud().await;
    let (service, kept, id) = with_account(
        &nextcloud,
        consent(vec![Scripted::AllowFirst(GrantScope::Always)]),
        vec![adding(nextcloud.base_url())],
    )
    .await;
    assert_eq!(id.as_str(), expected_id(&nextcloud, ""));

    // The account is stored, working, with its servers; the app password is filed and the
    // registry was written.
    let registry = service.registry();
    let [account] = registry.accounts.as_slice() else {
        panic!("one account: {:?}", registry.accounts);
    };
    assert_eq!(account.state, AccountState::Ok);
    assert_eq!(account.provider.as_str(), "nextcloud");
    assert_eq!(account.endpoints.len(), 4);
    assert!(
        registry.grants.is_empty(),
        "adding grants nothing to anyone"
    );
    assert_eq!(kept.store.stored().expect("saved").accounts.len(), 1);
    let password = kept
        .secrets
        .password(&id)
        .await
        .expect("the app password is filed");
    assert!(nextcloud.app_passwords().contains(&password));

    // A second app: nothing until it is allowed.
    let files = app("org.bardo.Files");
    assert_eq!(candidates_of(&service, &files), vec![]);
    let reply = service
        .handle(
            &files,
            AccountsRequest::Choose {
                need: storage(),
                class: DataClass::Files,
                usage: Usage::Interactive,
                window: ParentWindow::Unparented,
                account: None,
            },
        )
        .await;
    let AccountsReply::Chosen(chosen) = reply else {
        panic!("consent was accepted: {reply:?}");
    };
    assert_eq!(chosen.account, id);

    let found = candidates_of(&service, &files);
    let [candidate] = found.as_slice() else {
        panic!("one candidate: {found:?}");
    };
    let endpoints: Vec<(Family, String)> = candidate
        .endpoints
        .iter()
        .map(|e| (e.family, e.login.0.clone()))
        .collect();
    assert_eq!(endpoints, [(Family::WebDav, "alice".to_owned())]);
    assert!(
        candidate.endpoints[0]
            .url
            .as_str()
            .ends_with("/remote.php/dav/files/alice/")
    );
    assert_eq!(candidate.grant, chosen.grant);

    // The password is not in anything an app is told.
    assert!(!format!("{found:?}").contains(&password));

    // The audit log names the sign-in (the first app) and the grant (the second), no values.
    let entries = kept.audit.entries();
    assert!(
        entries
            .iter()
            .any(|e| e.event == AuditEvent::SignedIn && e.app == Some(app("org.bardo.Photos"))),
        "{entries:?}"
    );
    assert!(
        entries
            .iter()
            .any(|e| matches!(e.event, AuditEvent::Granted { .. }) && e.app == Some(files.clone()))
    );
    assert!(!format!("{entries:?}").contains(&password));
}

#[tokio::test]
async fn a_hint_skips_the_list_and_the_sheet_walks_the_sign_in() {
    let nextcloud = nextcloud().await;
    let sheets = TestSheets::new(vec![adding(nextcloud.base_url())], consent(vec![]));
    let shown = sheets.shown();
    let (service, _kept) = service(vec![provider()], sheets);
    let reply = service
        .handle(
            &app("org.bardo.Photos"),
            AccountsRequest::AddAccount {
                hint: ProviderHint::Provider(ProviderId::parse("nextcloud").expect("id")),
                window: ParentWindow::Unparented,
            },
        )
        .await;
    added(reply);
    let views: Vec<&str> = shown
        .lock()
        .expect("shown")
        .iter()
        .map(|v| match v {
            SheetView::Providers(_) => "providers",
            SheetView::Working { .. } => "working",
            SheetView::SignIn(_) => "form",
            SheetView::BrowserWait { .. } => "browser",
            SheetView::Review(_) => "review",
            SheetView::Done => "done",
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    // Working first (the provider is known), a form for the server, the page, the review, done;
    // each `Working` is the sign-in thinking between them.
    assert_eq!(views.first(), Some(&"working"));
    assert!(!views.contains(&"providers"), "{views:?}");
    let order: Vec<&&str> = views.iter().filter(|v| **v != "working").collect();
    assert_eq!(order, [&"form", &"browser", &"review", &"done"]);
}

#[tokio::test]
async fn a_hint_for_a_provider_that_is_not_installed_is_unavailable() {
    let (service, _kept) = service(vec![provider()], TestSheets::new(vec![], consent(vec![])));
    let reply = service
        .handle(
            &app("org.bardo.Photos"),
            AccountsRequest::AddAccount {
                hint: ProviderHint::Provider(ProviderId::parse("nope").expect("id")),
                window: ParentWindow::Unparented,
            },
        )
        .await;
    assert_eq!(reply, AccountsReply::Refused(Refusal::Unavailable));
}

#[tokio::test]
async fn the_person_may_take_their_time_in_the_browser() {
    let nextcloud = nextcloud().await;
    let mut person = adding(nextcloud.base_url());
    // The person opens the page only once the sign-in has polled twice, so it did wait.
    let seen = (*nextcloud).clone();
    person.opens_page = Some(Opens::once(move || {
        seen.hits()
            .iter()
            .filter(|h| h.target == "/index.php/login/v2/poll")
            .count()
            >= 2
    }));
    let (_service, _kept, id) = with_account(&nextcloud, consent(vec![]), vec![person]).await;
    assert_eq!(id.as_str(), expected_id(&nextcloud, ""));
    let polls = nextcloud
        .hits()
        .into_iter()
        .filter(|h| h.target == "/index.php/login/v2/poll")
        .count();
    assert!(polls > 1, "it waited: {polls} polls");
}

#[tokio::test]
async fn add_and_allow_is_one_step_with_one_grant() {
    let nextcloud = nextcloud().await;
    let (service, kept) = service(
        vec![provider()],
        TestSheets::new(vec![adding(nextcloud.base_url())], consent(vec![])),
    );
    let files = app("org.bardo.Files");
    let reply = service
        .add_and_allow(
            &files,
            ProviderHint::Any,
            ParentWindow::Unparented,
            AllowFor {
                need: storage(),
                class: DataClass::Files,
                usage: Usage::Interactive,
            },
        )
        .await;
    let id = added(reply);
    let grants = service.registry().grants;
    assert_eq!(grants.len(), 1);
    assert_eq!((&grants[0].key.app, &grants[0].key.account), (&files, &id));
    // No second prompt: the app finds the account at once.
    assert_eq!(candidates_of(&service, &files).len(), 1);
    let kinds: Vec<_> = kept
        .audit
        .entries()
        .iter()
        .map(|e| std::mem::discriminant(&e.event))
        .collect();
    assert_eq!(kinds.len(), 2, "SignedIn and Granted");
}

#[tokio::test]
async fn add_and_allow_grants_nothing_when_the_new_account_does_not_meet_the_need() {
    let nextcloud = nextcloud().await;
    let (service, _kept) = service(
        vec![provider()],
        TestSheets::new(vec![adding(nextcloud.base_url())], consent(vec![])),
    );
    let mail = Need::Mail(account_match::MailNeed::new(
        Access::ReadWrite,
        true,
        Delta::Poll,
    ));
    let reply = service
        .add_and_allow(
            &app("org.bardo.Mail"),
            ProviderHint::Any,
            ParentWindow::Unparented,
            AllowFor {
                need: mail,
                class: DataClass::Mail,
                usage: Usage::Interactive,
            },
        )
        .await;
    added(reply);
    assert!(service.registry().grants.is_empty());
}

#[tokio::test]
async fn removing_a_nextcloud_account_deletes_its_app_password_at_the_server_then_wipes_it() {
    let nextcloud = nextcloud().await;
    let (service, kept, id) = with_account(
        &nextcloud,
        consent(vec![]),
        vec![adding(nextcloud.base_url())],
    )
    .await;
    let password = kept.secrets.password(&id).await.expect("filed");
    assert!(nextcloud.app_passwords().contains(&password));

    service.remove_account(&id).await.expect("removed");

    assert!(
        !nextcloud.app_passwords().contains(&password),
        "the server no longer honours the app password"
    );
    assert!(service.registry().accounts.is_empty());
    assert_eq!(kept.secrets.password(&id).await, None);
}

fn choose_as(files: &AppId) -> (AppId, AccountsRequest) {
    (
        files.clone(),
        AccountsRequest::Choose {
            need: storage(),
            class: DataClass::Files,
            usage: Usage::Interactive,
            window: ParentWindow::Unparented,
            account: None,
        },
    )
}

#[tokio::test]
async fn add_account_on_the_alert_adds_and_allows_in_one_step_with_one_grant() {
    let nextcloud = nextcloud().await;
    // The account added from the alert is another server's: the same login twice is refused.
    let another = self::nextcloud().await;
    let (service, kept, first) = with_account(
        &nextcloud,
        consent(vec![Scripted::AddAccount]),
        vec![adding(nextcloud.base_url()), adding(another.base_url())],
    )
    .await;
    let (files, request) = choose_as(&app("org.bardo.Files"));
    let AccountsReply::Chosen(chosen) = service.handle(&files, request).await else {
        panic!("the new account is chosen");
    };
    assert_ne!(chosen.account, first, "the account is the one just added");
    let registry = service.registry();
    assert_eq!(registry.accounts.len(), 2);
    assert_eq!(registry.grants.len(), 1, "one grant, not a second prompt");
    let grant = &registry.grants[0];
    assert_eq!(
        (&grant.key.app, &grant.key.account),
        (&files, &chosen.account)
    );
    assert_eq!(grant.id, chosen.grant);
    let granted = kept
        .audit
        .entries()
        .iter()
        .filter(|e| matches!(e.event, AuditEvent::Granted { .. }))
        .count();
    assert_eq!(granted, 1);
}

#[tokio::test]
async fn cancelling_the_add_from_the_alert_grants_nothing() {
    let nextcloud = nextcloud().await;
    let mut gives_up = adding(nextcloud.base_url());
    gives_up.opens_page = None;
    gives_up.gives_up = true;
    let (service, _kept, _first) = with_account(
        &nextcloud,
        consent(vec![Scripted::AddAccount]),
        vec![adding(nextcloud.base_url()), gives_up],
    )
    .await;
    nextcloud.set_login_policy(LoginPolicy::Pending);
    let (files, request) = choose_as(&app("org.bardo.Files"));
    let reply = service.handle(&files, request).await;
    assert!(
        matches!(reply, AccountsReply::Refused(_)),
        "no account chosen: {reply:?}"
    );
    let registry = service.registry();
    assert_eq!(registry.accounts.len(), 1);
    assert!(
        registry.grants.is_empty(),
        "cancelling the add grants nothing"
    );
}
