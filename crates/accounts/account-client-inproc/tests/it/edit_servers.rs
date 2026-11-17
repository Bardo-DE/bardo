//! An app that hosts the service itself (mailo's shape) edits an IMAP account's servers through
//! `Porter::accounts().edit_servers`: the sheet is drawn by the app's own host, the typed servers
//! are logged in at a fake server before anything is handed back, and the app keeps what comes
//! back in its own store. Porter stores nothing.

use account_client::EditError;
use account_client_inproc::{HostConfig, hosted};
use account_core::{
    AccountId, AccountLabel, AppId, AppName, Isolation, ProviderId, SecretText, Tls,
};
use account_fake::{ScriptedSheets, mail_provider};
use account_fake_servers::net::{Bind, port_of};
use account_fake_servers::{
    Accounts as MailAccounts, FakeDns, FakeImap, FakeSmtp, Running, Secret, mailbox,
};
use account_families::DynProvider;
use account_family_generic::{GenericProvider, MailLookup};
use account_provider::parse_provider;
use account_secrets::MemorySecrets;
use account_sheet::{
    EditServers, FieldAnswer, FieldKind, FieldValue, Hop, MailServers, Manual, ServersFixed,
    SheetInput,
};
use account_wire::{ParentWindow, Refusal};
use http_core::SharedHttp;
use http_reqwest::ReqwestHttp;

const ADDRESS: &str = "ada@example.test";
const PASSWORD: &str = "s3cret-example";

fn app() -> AppId {
    AppId {
        name: AppName::parse("org.bardo.Mail").expect("app name"),
        isolation: Isolation::Flatpak,
    }
}

fn plain(kind: FieldKind, text: &str) -> FieldAnswer {
    FieldAnswer {
        kind,
        value: FieldValue::Plain(text.to_owned()),
    }
}

fn hop(host: &str, port: u16, security: Tls) -> Hop {
    Hop {
        host: host.to_owned(),
        port,
        security,
    }
}

/// What the app keeps of an account: its own store, not porter's.
#[derive(Debug, PartialEq)]
struct Kept {
    servers: Manual,
    name: Option<String>,
    password: Option<String>,
}

struct World {
    imap: Running<account_fake_servers::MailHandle>,
    smtp: Running<account_fake_servers::MailHandle>,
}

async fn world() -> World {
    let accounts = MailAccounts::password(ADDRESS, PASSWORD);
    World {
        imap: FakeImap::start(&Bind::Port(0), Tls::Plain, accounts.clone(), mailbox(1))
            .await
            .expect("imap"),
        smtp: FakeSmtp::start(&Bind::Port(0), Tls::Plain, accounts)
            .await
            .expect("smtp"),
    }
}

fn mail_family() -> Box<dyn DynProvider> {
    let spec = parse_provider(include_str!("../../../providers/generic-imap.toml"))
        .expect("the shipped file");
    DynProvider::boxed(GenericProvider::<MailLookup>::new(
        spec,
        SharedHttp::new(ReqwestHttp::new()),
        FakeDns::new(),
    ))
}

/// The account as the app stored it when it was added: servers that no longer answer.
fn stored_before() -> Kept {
    Kept {
        servers: Manual::Imap(MailServers {
            incoming: hop("imap.example.test", 993, Tls::Implicit),
            outgoing: hop("smtp.example.test", 465, Tls::Implicit),
            login: None,
        }),
        name: Some("Ada".to_owned()),
        password: Some(PASSWORD.to_owned()),
    }
}

fn request(kept: &Kept) -> EditServers {
    let mut edit = EditServers::new(
        AccountId::parse("mail-ada").expect("id"),
        ProviderId::parse("generic-imap").expect("provider id"),
        AccountLabel(ADDRESS.to_owned()),
        kept.servers.clone(),
    );
    if let Some(name) = &kept.name {
        edit = edit.with_name(name.clone());
    }
    if let Some(password) = &kept.password {
        edit = edit.keeping(SecretText::new(password.clone()));
    }
    edit
}

#[tokio::test]
async fn a_hosted_app_edits_an_imap_accounts_host_and_port_and_the_stored_account_changes() {
    let world = world().await;
    let (imap, smtp) = (port_of(world.imap.address()), port_of(world.smtp.address()));
    let typed = SheetInput::Submit(vec![
        plain(FieldKind::Name, "Ada Lovelace"),
        plain(FieldKind::Protocol, "imap"),
        plain(FieldKind::Server, "127.0.0.1"),
        plain(FieldKind::Security, "plain"),
        plain(FieldKind::Port, &imap.to_string()),
        plain(FieldKind::OutgoingServer, "127.0.0.1"),
        plain(FieldKind::OutgoingSecurity, "plain"),
        plain(FieldKind::OutgoingPort, &smtp.to_string()),
    ]);
    let sheets = ScriptedSheets::default().conversing([vec![typed]]);
    let porter = hosted(HostConfig::new(
        app(),
        vec![mail_family()],
        MemorySecrets::default(),
        sheets,
    ));

    let mut kept = stored_before();
    let edited = porter
        .accounts()
        .edit_servers(request(&kept), &ParentWindow::Unparented)
        .await
        .expect("the servers let the account in");

    // The app stores what came back; the password was not retyped, so it keeps its own.
    let (servers, name, password) = edited.into_parts();
    assert_eq!(password, None);
    kept.servers = servers;
    kept.name = name;
    assert_eq!(
        kept,
        Kept {
            servers: Manual::Imap(MailServers {
                incoming: hop("127.0.0.1", imap, Tls::Plain),
                outgoing: hop("127.0.0.1", smtp, Tls::Plain),
                login: None,
            }),
            name: Some("Ada Lovelace".to_owned()),
            password: Some(PASSWORD.to_owned()),
        }
    );
    // The check ran against the fake servers, with the password the app held, and no bearer.
    let attempts = [world.imap.attempts(), world.smtp.attempts()].concat();
    assert!(attempts.len() >= 2, "{attempts:?}");
    assert!(
        attempts.iter().all(|a| a.accepted && a.user == ADDRESS),
        "{attempts:?}"
    );
    assert!(
        attempts
            .iter()
            .all(|a| a.secret == Secret::Password(PASSWORD.into())),
        "{attempts:?}"
    );
}

#[tokio::test]
async fn servers_that_do_not_let_the_account_in_are_not_handed_back() {
    let world = world().await;
    let imap = port_of(world.imap.address());
    // The right host and a wrong port for SMTP; then the person closes the sheet.
    let typed = SheetInput::Submit(vec![
        plain(FieldKind::Protocol, "imap"),
        plain(FieldKind::Server, "127.0.0.1"),
        plain(FieldKind::Security, "plain"),
        plain(FieldKind::Port, &imap.to_string()),
        plain(FieldKind::OutgoingServer, "127.0.0.1"),
        plain(FieldKind::OutgoingSecurity, "plain"),
        plain(FieldKind::OutgoingPort, "1"),
    ]);
    let sheets = ScriptedSheets::default().conversing([vec![typed, SheetInput::Dismiss]]);
    let porter = hosted(HostConfig::new(
        app(),
        vec![mail_family()],
        MemorySecrets::default(),
        sheets,
    ));
    let got = porter
        .accounts()
        .edit_servers(request(&stored_before()), &ParentWindow::Unparented)
        .await;
    assert_eq!(got.err(), Some(EditError::Refused(Refusal::Dismissed)));
    assert!(world.smtp.attempts().is_empty());
}

#[tokio::test]
async fn a_cancelled_edit_hands_nothing_back() {
    let sheets = ScriptedSheets::default().conversing([vec![SheetInput::Dismiss]]);
    let porter = hosted(HostConfig::new(
        app(),
        vec![mail_family()],
        MemorySecrets::default(),
        sheets,
    ));
    let got = porter
        .accounts()
        .edit_servers(request(&stored_before()), &ParentWindow::Unparented)
        .await;
    assert_eq!(got.err(), Some(EditError::Refused(Refusal::Dismissed)));
}

#[tokio::test]
async fn an_account_whose_provider_signs_in_at_its_own_page_has_fixed_servers() {
    let porter = hosted(HostConfig::new(
        app(),
        vec![mail_provider()],
        MemorySecrets::default(),
        ScriptedSheets::default(),
    ));
    let edit = EditServers::new(
        AccountId::parse("mail-ada").expect("id"),
        ProviderId::parse("fake-mail").expect("provider id"),
        AccountLabel(ADDRESS.to_owned()),
        stored_before().servers,
    );
    let got = porter
        .accounts()
        .edit_servers(edit, &ParentWindow::Unparented)
        .await;
    assert_eq!(
        got.err(),
        Some(EditError::Fixed(ServersFixed::ProviderSignIn))
    );
}

#[test]
fn the_password_an_app_holds_shows_in_no_debug() {
    let edit = request(&stored_before());
    assert!(!format!("{edit:?}").contains(PASSWORD), "{edit:?}");
    let typed = vec![
        plain(FieldKind::Protocol, "imap"),
        plain(FieldKind::Server, "imap.example.test"),
        plain(FieldKind::Security, "tls"),
        plain(FieldKind::OutgoingServer, "smtp.example.test"),
        plain(FieldKind::OutgoingSecurity, "tls"),
        FieldAnswer {
            kind: FieldKind::Password,
            value: FieldValue::Secret(SecretText::new(PASSWORD)),
        },
    ];
    let edited = account_sheet::EditedServers::from_answers(&typed, Vec::new()).expect("reads");
    assert!(!format!("{edited:?}").contains(PASSWORD), "{edited:?}");
}
