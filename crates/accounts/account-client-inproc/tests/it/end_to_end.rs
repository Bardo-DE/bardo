//! An app on the in-process transport, over the fake providers: it finds an account by
//! capability, is asked for consent, and gets a token; a refusal sticks; a once-grant is spent.

use account_capability::{Access, Delta, StorageScope};
use account_client::{AccountsError, ConsentOffer, Found, GrantsError, NoAccount, Porter};
use account_client_inproc::Hosted;
use account_core::AccountId;
use account_core::consent::{GrantScope, Usage};
use account_core::{AppId, AppName, DataClass, Isolation, RelayAuth};
use account_fake::{FakeService, Scripted, ScriptedSheets, fake_service};
use account_match::Need;
use account_match::StorageNeed;
use account_wire::{ParentWindow, Refusal};
use std::sync::Arc;

type App = Porter<Hosted<FakeService>>;

fn app_id(name: &str) -> AppId {
    AppId {
        name: AppName::parse(name).expect("app name"),
        isolation: Isolation::Flatpak,
    }
}

fn app(service: &Arc<FakeService>, name: &str) -> App {
    Porter::over(Hosted::new(Arc::clone(service), app_id(name)))
}

fn storage(delta: Delta) -> Need {
    Need::Storage(StorageNeed::new(
        Access::ReadWrite,
        delta,
        StorageScope::AppFolder,
        false,
    ))
}

fn needs_consent(found: Found) -> ConsentOffer {
    match found {
        Found::NeedsConsent(offer) => offer,
        other => panic!("expected NeedsConsent, got {other:?}"),
    }
}

#[tokio::test]
async fn a_storage_need_finds_the_fake_cloud_after_consent() {
    let prompter = ScriptedSheets::answering([Scripted::AllowFirst(GrantScope::Always)]);
    let log = prompter.log();
    let service = Arc::new(fake_service(prompter).await);
    let photos = app(&service, "org.bardo.Photos");
    let need = storage(Delta::Poll);

    let offer = needs_consent(
        photos
            .grants()
            .find(&need, DataClass::Photos, Usage::Interactive)
            .await
            .expect("find"),
    );
    let chosen = photos
        .grants()
        .request_grant(&offer, &ParentWindow::Unparented)
        .await
        .expect("granted");

    let storage_id = AccountId::parse("fake-storage").expect("id");
    assert_eq!(chosen.account, storage_id);
    let offered: Vec<Vec<AccountId>> = log
        .asked()
        .iter()
        .map(|ask| ask.accounts.iter().map(|c| c.account.clone()).collect())
        .collect();
    assert_eq!(
        offered,
        vec![vec![storage_id.clone()]],
        "only the storage account is offered"
    );

    match photos
        .grants()
        .find(&need, DataClass::Photos, Usage::Interactive)
        .await
        .expect("find")
    {
        Found::One(candidate) => assert_eq!(candidate, chosen),
        other => panic!("expected One, got {other:?}"),
    }
    let plan = service
        .open_authenticated(
            &app_id("org.bardo.Photos"),
            &chosen.grant,
            &chosen.endpoints[0].url,
        )
        .await
        .expect("a relay is planned");
    assert!(
        matches!(&plan.auth, RelayAuth::Password(password) if password.expose() == "app-pw"),
        "{:?}",
        plan.auth
    );

    let files = app(&service, "org.bardo.Files");
    let other = files
        .grants()
        .find(&need, DataClass::Photos, Usage::Interactive)
        .await
        .expect("find");
    assert!(
        matches!(other, Found::NeedsConsent(_)),
        "another app's grant is not shared: {other:?}"
    );
}

#[tokio::test]
async fn a_refusal_sticks_for_that_app() {
    let service = Arc::new(fake_service(ScriptedSheets::answering([Scripted::Deny])).await);
    let photos = app(&service, "org.bardo.Photos");
    let need = storage(Delta::Poll);
    let offer = needs_consent(
        photos
            .grants()
            .find(&need, DataClass::Photos, Usage::Interactive)
            .await
            .expect("find"),
    );
    let refused = photos
        .grants()
        .request_grant(&offer, &ParentWindow::Unparented)
        .await;
    assert_eq!(refused, Err(GrantsError::Refused(Refusal::Denied)));
    let after = photos
        .grants()
        .find(&need, DataClass::Photos, Usage::Interactive)
        .await
        .expect("find");
    assert_eq!(after, Found::None(NoAccount::Denied));
}

#[tokio::test]
async fn a_need_no_provider_meets_is_unsupported() {
    let service = Arc::new(fake_service(ScriptedSheets::default()).await);
    let photos = app(&service, "org.bardo.Photos");
    let found = photos
        .grants()
        .find(&storage(Delta::Push), DataClass::Photos, Usage::Interactive)
        .await
        .expect("find");
    assert_eq!(found, Found::None(NoAccount::Unsupported));
}

#[tokio::test]
async fn a_once_grant_is_spent_by_its_first_relay() {
    let service = Arc::new(
        fake_service(ScriptedSheets::answering([Scripted::AllowFirst(
            GrantScope::Once,
        )]))
        .await,
    );
    let photos = app(&service, "org.bardo.Photos");
    let need = storage(Delta::Poll);
    let offer = needs_consent(
        photos
            .grants()
            .find(&need, DataClass::Files, Usage::Interactive)
            .await
            .expect("find"),
    );
    let chosen = photos
        .grants()
        .request_grant(&offer, &ParentWindow::Unparented)
        .await
        .expect("granted");
    let me = app_id("org.bardo.Photos");
    let relay = |grant| service.open_authenticated(&me, grant, &chosen.endpoints[0].url);
    relay(&chosen.grant).await.expect("first relay");
    let again = relay(&chosen.grant).await.map(|_| ());
    assert_eq!(again, Err(Refusal::UnknownGrant));
    let after = photos
        .grants()
        .find(&need, DataClass::Files, Usage::Interactive)
        .await
        .expect("find");
    assert!(matches!(after, Found::NeedsConsent(_)), "{after:?}");
}

#[tokio::test]
async fn revoking_a_grant_ends_it() {
    let service = Arc::new(
        fake_service(ScriptedSheets::answering([Scripted::AllowFirst(
            GrantScope::Always,
        )]))
        .await,
    );
    let photos = app(&service, "org.bardo.Photos");
    let need = storage(Delta::Poll);
    let offer = needs_consent(
        photos
            .grants()
            .find(&need, DataClass::Photos, Usage::Interactive)
            .await
            .expect("find"),
    );
    let chosen = photos
        .grants()
        .request_grant(&offer, &ParentWindow::Unparented)
        .await
        .expect("granted");
    let me = app_id("org.bardo.Photos");
    assert_eq!(service.grants(&me).len(), 1);
    service.revoke(&me, &chosen.grant).await.expect("revoke");
    assert_eq!(service.grants(&me), vec![]);
}

/// A DAV account added through a real family (the generic one, over a fake DAV server) from the
/// app's own call, then granted, then signed in again: the whole path of `AddAccount` and
/// `Reauthenticate` over the in-process carrier.
#[tokio::test]
async fn an_app_adds_an_account_through_a_family_is_granted_it_and_signs_it_in_again() {
    use account_capability::KindToggle;
    use account_capability::{Access, Delta};
    use account_core::ProviderId;
    use account_core::SecretText;
    use account_fake::NOW;
    use account_fake_servers::{FakeDav, FakeDns};
    use account_families::DynProvider;
    use account_family_generic::{DavLookup, GenericProvider};
    use account_http::SharedHttp;
    use account_http_reqwest::ReqwestHttp;
    use account_match::PimNeed;
    use account_provider::parse_provider;
    use account_secrets::MemorySecrets;
    use account_service::AccountService;
    use account_service_core::registry::Registry;
    use account_sheet::{FieldAnswer, FieldKind, FieldValue, SheetInput};
    use base_time::FixedClock;

    let dav = FakeDav::start("bob", "hunter2").await.expect("dav");
    let spec = parse_provider(include_str!("../../../providers/generic-dav.toml")).expect("file");
    let provider = DynProvider::boxed(GenericProvider::<DavLookup>::new(
        spec,
        SharedHttp::new(ReqwestHttp::new()),
        FakeDns::new(),
    ));
    let form = |password: &str| {
        SheetInput::Submit(vec![
            FieldAnswer {
                kind: FieldKind::Server,
                value: FieldValue::Plain(format!("{}/dav/calendar/", dav.base_url())),
            },
            FieldAnswer {
                kind: FieldKind::Username,
                value: FieldValue::Plain("bob".into()),
            },
            FieldAnswer {
                kind: FieldKind::Password,
                value: FieldValue::Secret(SecretText::new(password)),
            },
        ])
    };
    let review = SheetInput::Confirm(vec![
        KindToggle {
            kind: account_capability::CapabilityKind::Calendar,
            toggle: true,
        },
        KindToggle {
            kind: account_capability::CapabilityKind::Contacts,
            toggle: false,
        },
    ]);
    let sheets =
        ScriptedSheets::answering([Scripted::AllowFirst(GrantScope::Always)]).conversing([
            // The add: pick the provider, fill the form, confirm the review.
            vec![
                SheetInput::Pick(ProviderId::parse("generic-dav").expect("id")),
                form("hunter2"),
                review,
            ],
            // Signing in again: the form, and no review.
            vec![form("hunter2")],
            // A password the server refuses.
            vec![form("wrong"), SheetInput::Dismiss],
        ]);
    let service = Arc::new(AccountService::new(
        vec![provider],
        Registry::default(),
        MemorySecrets::default(),
        sheets,
        FixedClock(NOW),
    ));
    let calendar = app_over(&service, "org.bardo.Calendar");

    let added = calendar
        .accounts()
        .add(account_core::ProviderHint::Any, &ParentWindow::Unparented)
        .await
        .expect("added");
    assert_eq!(added.as_str(), "generic-dav-bob-127.0.0.1");

    let need = Need::Calendar(PimNeed::new(Access::ReadWrite, Delta::Poll));
    let offer = needs_consent(
        calendar
            .grants()
            .find(&need, DataClass::Calendar, Usage::Interactive)
            .await
            .expect("find"),
    );
    let chosen = calendar
        .grants()
        .request_grant(&offer, &ParentWindow::Unparented)
        .await
        .expect("granted");
    assert_eq!(chosen.account, added);
    assert_eq!(
        chosen.endpoints.len(),
        1,
        "only the CalDav endpoint serves a calendar"
    );

    calendar
        .accounts()
        .reauthenticate(&added, &ParentWindow::Unparented)
        .await
        .expect("signed in again");
    assert_eq!(
        calendar
            .accounts()
            .reauthenticate(&added, &ParentWindow::Unparented)
            .await,
        Err(AccountsError::Refused(Refusal::Denied)),
        "the server refused the password"
    );
    // Another app was never granted the account, so it cannot sign it in again.
    let other = app_over(&service, "org.bardo.Other");
    assert_eq!(
        other
            .accounts()
            .reauthenticate(&added, &ParentWindow::Unparented)
            .await,
        Err(AccountsError::Refused(Refusal::UnknownGrant))
    );
}

/// The mailo ask: an app adds an account that is already here (the same login at the same
/// server). The sheet says so at the review and stores nothing, and the app is told which
/// account it is, so it can go on to ask for a grant of it.
#[tokio::test]
async fn adding_an_account_that_is_already_here_tells_the_app_which_one_it_is() {
    use account_capability::KindToggle;
    use account_core::ProviderId;
    use account_core::SecretText;
    use account_fake::NOW;
    use account_fake_servers::{FakeDav, FakeDns};
    use account_families::DynProvider;
    use account_family_generic::{DavLookup, GenericProvider};
    use account_http::SharedHttp;
    use account_http_reqwest::ReqwestHttp;
    use account_provider::parse_provider;
    use account_secrets::MemorySecrets;
    use account_service::AccountService;
    use account_service_core::registry::Registry;
    use account_sheet::{FieldAnswer, FieldKind, FieldValue, SheetInput};
    use base_time::FixedClock;

    let dav = FakeDav::start("bob", "hunter2").await.expect("dav");
    let spec = parse_provider(include_str!("../../../providers/generic-dav.toml")).expect("file");
    let provider = DynProvider::boxed(GenericProvider::<DavLookup>::new(
        spec,
        SharedHttp::new(ReqwestHttp::new()),
        FakeDns::new(),
    ));
    let pick = SheetInput::Pick(ProviderId::parse("generic-dav").expect("id"));
    let form = SheetInput::Submit(vec![
        FieldAnswer {
            kind: FieldKind::Server,
            value: FieldValue::Plain(format!("{}/dav/calendar/", dav.base_url())),
        },
        FieldAnswer {
            kind: FieldKind::Username,
            value: FieldValue::Plain("bob".into()),
        },
        FieldAnswer {
            kind: FieldKind::Password,
            value: FieldValue::Secret(SecretText::new("hunter2")),
        },
    ]);
    let review = SheetInput::Confirm(vec![KindToggle {
        kind: account_capability::CapabilityKind::Calendar,
        toggle: true,
    }]);
    let sheets = ScriptedSheets::answering([]).conversing([
        vec![pick.clone(), form.clone(), review],
        // The same login again: the sheet stops at the review and says it is there already; the
        // person closes it.
        vec![pick, form, SheetInput::Dismiss],
    ]);
    let service = Arc::new(AccountService::new(
        vec![provider],
        Registry::default(),
        MemorySecrets::default(),
        sheets,
        FixedClock(NOW),
    ));
    let calendar = app_over(&service, "org.bardo.Calendar");
    let added = calendar
        .accounts()
        .add(account_core::ProviderHint::Any, &ParentWindow::Unparented)
        .await
        .expect("added");
    let held = service.registry().accounts.clone();

    assert_eq!(
        calendar
            .accounts()
            .add(account_core::ProviderHint::Any, &ParentWindow::Unparented)
            .await,
        Err(AccountsError::AlreadyAdded(added))
    );
    assert_eq!(service.registry().accounts, held, "nothing was added");
}

/// An HTTP client that finds nothing anywhere: every address publishes no server.
#[derive(Debug, Clone)]
struct NothingPublished;

impl account_http::Http for NothingPublished {
    #[expect(clippy::unused_async_trait_impl, reason = "the trait method is async; this implementation has nothing to await")]
    async fn send(
        &self,
        _request: account_http::http::Request<Vec<u8>>,
    ) -> Result<account_http::http::Response<Vec<u8>>, account_http::HttpError> {
        let mut response = account_http::http::Response::new(Vec::new());
        *response.status_mut() = account_http::http::StatusCode::NOT_FOUND;
        Ok(response)
    }
}

/// A mail address whose domain publishes no server: the sheet asks for the server by hand, and a
/// POP3 account typed there is added, then found by a mail need with its POP3 and SMTP
/// endpoints and a login name that is not the address.
#[tokio::test]
async fn a_typed_pop3_server_is_added_through_the_sheet_and_found_by_a_mail_need() {
    use account_capability::KindToggle;
    use account_core::ProviderId;
    use account_core::{Family, SecretText};
    use account_fake::NOW;
    use account_fake_servers::net::{Bind, port_of};
    use account_fake_servers::{Accounts as MailAccounts, FakeDns, FakePop3, mailbox};
    use account_families::DynProvider;
    use account_family_generic::{GenericProvider, MailLookup};
    use account_http::SharedHttp;
    use account_match::MailNeed;
    use account_provider::parse_provider;
    use account_secrets::MemorySecrets;
    use account_service::AccountService;
    use account_service_core::registry::Registry;
    use account_sheet::{FieldAnswer, FieldKind, FieldValue, SheetInput};
    use base_time::FixedClock;

    // The password is tried at the POP3 server before the review, so one is running.
    let pop3 = FakePop3::start(
        &Bind::Port(0),
        account_core::Tls::Plain,
        MailAccounts::password("ada.login", "s3cret"),
        mailbox(1),
    )
    .await
    .expect("pop3");
    let pop3_port = port_of(pop3.address());
    let plain = |kind, text: &str| FieldAnswer {
        kind,
        value: FieldValue::Plain(text.to_owned()),
    };
    let spec = parse_provider(include_str!("../../../providers/generic-imap.toml")).expect("file");
    let provider = DynProvider::boxed(GenericProvider::<MailLookup>::new(
        spec,
        SharedHttp::new(NothingPublished),
        FakeDns::new(),
    ));
    let first = SheetInput::Submit(vec![
        plain(FieldKind::Address, "ada@old-isp.example"),
        FieldAnswer {
            kind: FieldKind::Password,
            value: FieldValue::Secret(SecretText::new("s3cret")),
        },
    ]);
    let manual = SheetInput::Submit(vec![
        plain(FieldKind::Protocol, "pop3"),
        plain(FieldKind::Server, "127.0.0.1"),
        plain(FieldKind::Security, "plain"),
        plain(FieldKind::Port, &pop3_port.to_string()),
        plain(FieldKind::OutgoingServer, "smtp.old-isp.example"),
        plain(FieldKind::OutgoingSecurity, "starttls"),
        plain(FieldKind::OutgoingPort, "2525"),
        plain(FieldKind::Username, "ada.login"),
    ]);
    let review = SheetInput::Confirm(vec![KindToggle {
        kind: account_capability::CapabilityKind::Mail,
        toggle: true,
    }]);
    let sheets =
        ScriptedSheets::answering([Scripted::AllowFirst(GrantScope::Always)]).conversing([vec![
            SheetInput::Pick(ProviderId::parse("generic-imap").expect("id")),
            first,
            manual,
            review,
        ]]);
    let service = Arc::new(AccountService::new(
        vec![provider],
        Registry::default(),
        MemorySecrets::default(),
        sheets,
        FixedClock(NOW),
    ));
    let mail = app_over(&service, "org.bardo.Mail");
    let added = mail
        .accounts()
        .add(account_core::ProviderHint::Any, &ParentWindow::Unparented)
        .await
        .expect("added");

    let need = Need::Mail(MailNeed::new(Access::Read, true, Delta::Poll));
    let offer = needs_consent(
        mail.grants()
            .find(&need, DataClass::Mail, Usage::Interactive)
            .await
            .expect("find"),
    );
    let chosen = mail
        .grants()
        .request_grant(&offer, &ParentWindow::Unparented)
        .await
        .expect("granted");
    assert_eq!(chosen.account, added);
    let shown: Vec<_> = chosen
        .endpoints
        .iter()
        .map(|e| (e.family, e.url.to_string(), e.login.0.clone()))
        .collect();
    assert_eq!(
        shown,
        [
            (
                Family::Pop3,
                format!("pop3://127.0.0.1:{pop3_port}"),
                "ada.login".to_owned()
            ),
            (
                Family::Smtp,
                "smtp://smtp.old-isp.example:2525".to_owned(),
                "ada.login".to_owned()
            ),
        ]
    );
}

type FamilyService = account_service::AccountService<
    Box<dyn account_families::DynProvider>,
    account_secrets::MemorySecrets,
    ScriptedSheets,
    base_time::FixedClock,
>;

type FamilyApp = Porter<Hosted<FamilyService>>;

fn app_over(service: &Arc<FamilyService>, name: &str) -> FamilyApp {
    let id = AppId {
        name: AppName::parse(name).expect("app name"),
        isolation: Isolation::Flatpak,
    };
    Porter::over(Hosted::new(Arc::clone(service), id))
}
