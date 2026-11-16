//! inferd across a Tailscale network, end to end on private buses: two inferds, each in a world of
//! its own (its own bus, scratch directory, fake Tailscale and fake accountd), `pi` that lends
//! and `desk` that calls. Every address is a loopback address of the test's own, so a connection
//! from the one's address to the other's is what the network would carry; nothing leaves the
//! computer, and nothing touches the real Tailscale.

use crate::hosting;

use account_capability::ModelId;
use account_core::{AppId, AppName, Isolation};
use account_dbus::Details;
use hosting::accountd::{FakeAccount, Standing};
use hosting::bus::within;
use hosting::engine::{Chat, Script};
use hosting::entries;
use hosting::lab::Lab;
use hosting::rig::{Hosted, Plan, Trust, World};
use hosting::tailnet::{Addresses, TailnetPlan, addresses, free_port, network};
use inference_daemon::attached::{Attached, Place, Reach};
use inference_daemon::peers::Role;
use inference_dbus::inference::GUEST_KEY_STATE;
use inference_dbus::inference::InferenceProxy;
use std::net::{IpAddr, SocketAddr};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpSocket;

const COMPANION: &str = "org.bardo.Companion";

fn app(name: &str) -> AppId {
    AppId {
        name: AppName::parse(name).expect("name"),
        isolation: Isolation::Unsandboxed,
    }
}

/// `pi` (lends) and `desk` (calls), and what is needed to look at both.
struct Pair {
    pi: World,
    desk: World,
    a: Addresses,
    port: u16,
    _lab: Lab,
}

/// The catalogue both computers share: a model started by inferd (`tiny-chat`) and one served by
/// an engine of the person's own (`ATTACHED`).
fn catalog() -> Vec<(&'static str, String)> {
    vec![
        ("a-attached.toml", entries::attached()),
        ("b-chat.toml", entries::chat()),
        ("c-claude.toml", entries::claude()),
    ]
}

fn policy() -> inference_core::Policy {
    inference_core::Policy {
        local_only: false,
        floors: Vec::new(),
    }
}

/// `pi` lends (when `serving`) the two models it runs; `desk` has no
/// engine programs, as a laptop does not, and sees `pi` as one of its own.
async fn pair(name: &str, serving: bool) -> Pair {
    let a = addresses();
    let port = free_port();
    let dir = base_testkit::Scratch::new(&format!("tn-{name}"));
    let lab = Lab::start_in(dir, "lab", &[entries::SERVED], None).await;
    let pi = World::start(Plan {
        catalog: catalog(),
        scripts: vec![(
            "tiny-chat",
            Script {
                chat: vec![Chat::Say(vec!["from pi"])],
                dims: 0,
            },
        )],
        attached: vec![Attached {
            id: ModelId::parse(entries::ATTACHED).expect("id"),
            reach: Reach::Socket(lab.socket()),
            key_file: None,
            place: Place::ThisDevice,
            computer: None,
        }],
        policy: policy(),
        role: Role::Shell,
        app: Some(app("org.bardo.Shell")),
        tailnet: Some(TailnetPlan {
            network: network(("nPI", "pi", a.pi), ("nDESK", "desk", a.desk)),
            port,
            serving,
        }),
        ..Plan::default()
    })
    .await;
    let desk = World::start(Plan {
        catalog: catalog(),
        engine_programs: false,
        policy: policy(),
        role: Role::Placer,
        app: Some(app(COMPANION)),
        tailnet: Some(TailnetPlan {
            network: network(("nDESK", "desk", a.desk), ("nPI", "pi", a.pi)),
            port,
            serving: false,
        }),
        ..Plan::default()
    })
    .await;
    Pair {
        pi,
        desk,
        a,
        port,
        _lab: lab,
    }
}

/// Waits until `pi` listens at its address.
async fn listening(world: &World, at: SocketAddr) {
    let mut now = world.tailnet.as_ref().expect("tailnet").listening();
    within("pi to listen", now.wait_for(|now| now.bound.contains(&at)))
        .await
        .expect("listening");
}

async fn proxy(connection: &zbus::Connection) -> InferenceProxy<'_> {
    InferenceProxy::new(connection).await.expect("proxy")
}

/// The name of the error a call came back with.
fn error_name(error: &zbus::Error) -> String {
    match error {
        zbus::Error::MethodError(name, _, _) => name.to_string(),
        other => format!("{other:?}"),
    }
}

fn computer_error(name: &str) -> String {
    format!("{}{name}", inference_dbus::inference::COMPUTER_ERROR_PREFIX)
}

fn text_of(details: &Details, key: &str) -> Option<String> {
    String::try_from(details.get(key)?.try_clone().ok()?).ok()
}

/// A request sent to `to` from the address `source`, and the whole answer.
async fn http(source: IpAddr, to: SocketAddr, request: &str) -> String {
    let socket = TcpSocket::new_v4().expect("socket");
    socket.bind(SocketAddr::new(source, 0)).expect("bind");
    let mut stream = within("a connection", socket.connect(to))
        .await
        .expect("connect");
    stream.write_all(request.as_bytes()).await.expect("send");
    let mut answer = Vec::new();
    within("the answer", stream.read_to_end(&mut answer))
        .await
        .ok();
    String::from_utf8_lossy(&answer).into_owned()
}

const HELLO: &str = "GET /hello HTTP/1.1\r\nHost: pi\r\n\r\n";

fn chat_body(model: &str) -> String {
    let body = format!(r#"{{"model":"{model}","messages":[{{"role":"user","content":"hi"}}]}}"#);
    format!(
        "POST /v1/chat/completions HTTP/1.1\r\nHost: pi\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

/// `desk` adds `pi` as Settings does, and answers who is there.
async fn add_pi(pair: &Pair) -> String {
    let (settings, _) = pair
        .desk
        .connect_as(Role::Settings, app("org.bardo.Settings"))
        .await;
    let rows = proxy(&settings)
        .await
        .candidates()
        .await
        .expect("candidates");
    assert_eq!(rows.len(), 1, "{rows:?}");
    proxy(&settings)
        .await
        .add_tailnet_computer(&rows[0].0)
        .await
        .expect("added")
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cloud_account_is_never_reached_from_a_peer_even_when_it_is_the_only_engine() {
    // pi has nothing of its own to run and one cloud account that works: it lends nothing.
    let a = addresses();
    let port = free_port();
    let pi = World::start(Plan {
        catalog: vec![("c-claude.toml", entries::claude())],
        policy: policy(),
        role: Role::Shell,
        app: Some(app("org.bardo.Shell")),
        hosted: Some(Hosted {
            accounts: vec![FakeAccount {
                id: "openrouter",
                standing: Standing::Granted {
                    to: vec![COMPANION, "org.bardo.Guest", "org.bardo.Guest.nDESK"],
                    grant: "grant-openrouter",
                    key: Some("sk-or-test-key"),
                },
            }],
            chat: Script {
                chat: vec![Chat::Say(vec!["cloud"])],
                dims: 0,
            },
            trust: Trust::ScratchCa,
        }),
        tailnet: Some(TailnetPlan {
            network: network(("nPI", "pi", a.pi), ("nDESK", "desk", a.desk)),
            port,
            serving: true,
        }),
        ..Plan::default()
    })
    .await;
    let at = SocketAddr::new(a.pi, port);
    listening(&pi, at).await;
    // The person said yes to desk ahead of time, so only what is lent can stop the request.
    let (settings, _) = pi
        .connect_as(Role::Settings, app("org.bardo.Settings"))
        .await;
    proxy(&settings)
        .await
        .answer_guest("nDESK", inference_dbus::inference::GUEST_ANSWER_ALLOW)
        .await
        .expect("yes");
    let hello = http(a.desk, at, HELLO).await;
    assert!(hello.contains(r#""models":[]"#), "{hello}");
    let listed = http(a.desk, at, "GET /v1/models HTTP/1.1\r\nHost: pi\r\n\r\n").await;
    assert!(listed.contains(r#""data":[]"#), "{listed}");
    for model in [
        "claude-opus-5.5",
        "gpt-6-luna",
        "openrouter/claude-opus-5.5",
    ] {
        let said = http(a.desk, at, &chat_body(model)).await;
        assert!(said.starts_with("HTTP/1.1 404"), "{model}: {said}");
    }
    assert!(
        pi.provider
            .as_ref()
            .expect("provider")
            .requests()
            .is_empty(),
        "nothing was sent to the account"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_listener_follows_the_address_tailscale_gives_the_computer() {
    let p = pair("address", true).await;
    let first = SocketAddr::new(p.a.pi, p.port);
    listening(&p.pi, first).await;
    let moved = SocketAddr::new(p.a.spare, p.port);
    p.pi.tailscale
        .as_ref()
        .unwrap()
        .fake
        .edit(|net| net.me.addresses = vec![p.a.spare.to_string()]);
    listening(&p.pi, moved).await;
    let mut now = p.pi.tailnet.as_ref().unwrap().listening();
    within(
        "the old address to go",
        now.wait_for(|now| !now.bound.contains(&first)),
    )
    .await
    .expect("goes");
    within("the old address to refuse", async {
        while tokio::net::TcpStream::connect(first).await.is_ok() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await;
    // And when Tailscale stops, so does the listener.
    p.pi.tailscale.as_ref().unwrap().fake.stop();
    within(
        "listening to stop",
        now.wait_for(|now| now.bound.is_empty()),
    )
    .await
    .expect("stops");
}

#[tokio::test(flavor = "multi_thread")]
async fn candidates_are_looked_at_only_when_read_at_most_once_a_minute_and_only_with_an_account() {
    let p = pair("looks", true).await;
    listening(&p.pi, SocketAddr::new(p.a.pi, p.port)).await;
    let (settings, _) = p
        .desk
        .connect_as(Role::Settings, app("org.bardo.Settings"))
        .await;
    let settings = proxy(&settings).await;
    // Nobody read the list: nobody was looked at.
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_eq!(p.pi.tailscale.as_ref().unwrap().whois_asked(), 0);
    // Read twice, one look.
    assert_eq!(settings.candidates().await.expect("candidates").len(), 1);
    assert_eq!(settings.candidates().await.expect("candidates").len(), 1);
    assert_eq!(p.pi.tailscale.as_ref().unwrap().whois_asked(), 1);

    // With no Tailscale account in the accounts, accountd lists no computers: no look is made.
    let q = pair("no-account", true).await;
    listening(&q.pi, SocketAddr::new(q.a.pi, q.port)).await;
    q.desk
        .tailscale
        .as_ref()
        .unwrap()
        .account
        .store(false, std::sync::atomic::Ordering::SeqCst);
    let (settings, _) = q
        .desk
        .connect_as(Role::Settings, app("org.bardo.Settings"))
        .await;
    let settings = proxy(&settings).await;
    assert!(settings.candidates().await.expect("candidates").is_empty());
    assert_eq!(q.pi.tailscale.as_ref().unwrap().whois_asked(), 0);
    let refused = settings
        .add_tailnet_computer("nPI")
        .await
        .expect_err("refused");
    assert_eq!(error_name(&refused), computer_error("NotOnTailscale"));
}

#[tokio::test(flavor = "multi_thread")]
async fn adding_a_computer_that_is_not_there_or_twice_is_refused_in_plain_words() {
    let p = pair("adding", true).await;
    listening(&p.pi, SocketAddr::new(p.a.pi, p.port)).await;
    let (settings, _) = p
        .desk
        .connect_as(Role::Settings, app("org.bardo.Settings"))
        .await;
    let settings = proxy(&settings).await;
    for (node, name) in [("nNOBODY", "NotOnTailscale"), ("nDESK", "NotOnTailscale")] {
        let refused = settings
            .add_tailnet_computer(node)
            .await
            .expect_err("refused");
        assert_eq!(error_name(&refused), computer_error(name), "{node}");
    }
    let invalid = settings
        .add_tailnet_computer("not a node!")
        .await
        .expect_err("refused");
    assert_eq!(
        error_name(&invalid),
        "org.freedesktop.DBus.Error.InvalidArgs"
    );
    assert_eq!(
        settings.add_tailnet_computer("nPI").await.expect("added"),
        "computer:pi"
    );
    // Once added it is no longer offered, and adding it again is refused.
    assert!(settings.candidates().await.expect("candidates").is_empty());
    let twice = settings
        .add_tailnet_computer("nPI")
        .await
        .expect_err("refused");
    assert_eq!(error_name(&twice), computer_error("AlreadyThere"));
    // Removing it takes it away; it is offered again.
    settings
        .remove_computer("computer:pi")
        .await
        .expect("removed");
    let places = p.desk.accounts.places().await.expect("places");
    assert!(places.iter().all(|row| row.id.as_str() != "computer:pi"));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_name_follows_tailscale_and_the_place_stays() {
    let p = pair("renamed", true).await;
    listening(&p.pi, SocketAddr::new(p.a.pi, p.port)).await;
    add_pi(&p).await;
    p.desk
        .tailscale
        .as_ref()
        .unwrap()
        .fake
        .edit(|net| net.peers[0].name = "kitchen-pi".to_owned());
    let places = p.desk.accounts.places().await.expect("places");
    let row = places
        .iter()
        .find(|r| r.id.as_str() == "computer:pi")
        .expect("same place");
    assert_eq!(row.name, "kitchen-pi");
}

#[tokio::test(flavor = "multi_thread")]
async fn only_settings_may_add_or_look_and_only_settings_and_the_shell_may_answer() {
    let p = pair("callers", true).await;
    listening(&p.pi, SocketAddr::new(p.a.pi, p.port)).await;
    let denied = "org.freedesktop.DBus.Error.AccessDenied";
    // On desk: an app, the shell and the companion may not look at or add computers.
    for (role, who) in [
        (Role::App, "org.bardo.Memory"),
        (Role::Shell, "org.bardo.Shell"),
        (Role::Placer, COMPANION),
    ] {
        let (connection, _) = p.desk.connect_as(role, app(who)).await;
        let inference = proxy(&connection).await;
        let e = inference.candidates().await.expect_err("refused");
        assert_eq!(error_name(&e), denied, "{who}");
        let e = inference
            .add_tailnet_computer("nPI")
            .await
            .expect_err("refused");
        assert_eq!(error_name(&e), denied, "{who}");
        let e = inference.forget_guest("nPI").await.expect_err("refused");
        assert_eq!(error_name(&e), denied, "{who}");
    }
    // On pi: an app and the companion may neither see nor answer the computers that ask.
    for (role, who) in [(Role::App, "org.bardo.Memory"), (Role::Placer, COMPANION)] {
        let (connection, _) = p.pi.connect_as(role, app(who)).await;
        let inference = proxy(&connection).await;
        let e = inference.guests().await.expect_err("refused");
        assert_eq!(error_name(&e), denied, "{who}");
        let e = inference
            .answer_guest("nDESK", inference_dbus::inference::GUEST_ANSWER_ALLOW)
            .await
            .expect_err("refused");
        assert_eq!(error_name(&e), denied, "{who}");
    }
    // The shell may answer only a computer that is asking.
    let shell = proxy(&p.pi.client).await;
    let e = shell
        .answer_guest("nDESK", inference_dbus::inference::GUEST_ANSWER_ALLOW)
        .await
        .expect_err("nobody asked");
    assert_eq!(error_name(&e), computer_error("NotAsking"));
    let e = shell.forget_guest("nDESK").await.expect_err("refused");
    assert_eq!(error_name(&e), denied);
    // Settings at pi may answer one that never asked, if it is on the person's network.
    let (settings, _) =
        p.pi.connect_as(Role::Settings, app("org.bardo.Settings"))
            .await;
    let settings = proxy(&settings).await;
    settings
        .answer_guest("nDESK", inference_dbus::inference::GUEST_ANSWER_ALLOW)
        .await
        .expect("allowed");
    let e = settings
        .answer_guest("nNOBODY", inference_dbus::inference::GUEST_ANSWER_ALLOW)
        .await
        .expect_err("not there");
    assert_eq!(error_name(&e), computer_error("NotOnTailscale"));
    // The answer is a word: any other is invalid args, and nothing changes.
    for word in ["", "yes", "true", "Allow", "allow ", "maybe"] {
        let e = settings
            .answer_guest("nDESK", word)
            .await
            .expect_err("not an answer");
        assert_eq!(
            error_name(&e),
            "org.freedesktop.DBus.Error.InvalidArgs",
            "{word:?}"
        );
    }
    settings
        .answer_guest("nDESK", inference_dbus::inference::GUEST_ANSWER_DENY)
        .await
        .expect("denied");
    let rows = settings.guests().await.expect("guests");
    assert_eq!(
        text_of(&rows[0].1, GUEST_KEY_STATE).as_deref(),
        Some("denied")
    );
}
