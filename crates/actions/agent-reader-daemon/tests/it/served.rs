//! readerd serves `org.bardo.Reader1` as declared: the interface on the bus is the one in
//! `dbus/org.bardo.Reader1.xml`, and the name is claimed once.

use crate::support::bus::PrivateBus;
use crate::support::inferd::ScriptedInferd;
use account_core::{AppId, AppName, Isolation};
use action_client::{InProcess, Intents};
use action_core::AgentConfig;
use action_dbus::callers::Identity;
use action_dbus::{Bus, BusConnection, introspection};
use action_fake::fake_router;
use action_vocab::CallerRole;
use action_wire::CallerId;
use agent_reader_daemon::{ReaderHost, ReaderService, ServeError, serve_on};
use std::collections::BTreeSet;
use std::sync::Arc;

pub(crate) fn service() -> Arc<ReaderService<ScriptedInferd, InProcess<action_fake::FakeSeams>>> {
    let router = Arc::new(fake_router(AgentConfig::default()).expect("router"));
    let reader = CallerId {
        app: AppId {
            name: AppName::parse("org.bardo.Reader1").expect("app"),
            isolation: Isolation::Unsandboxed,
        },
        roles: BTreeSet::from([CallerRole::Reader]),
    };
    Arc::new(ReaderService::new(
        ReaderHost::start(),
        ScriptedInferd::away(),
        Intents::over(InProcess::new(router, reader)),
    ))
}

/// Who readerd would answer on `connection`: the shipped table over an empty fake proc root.
fn nobody(connection: &BusConnection, scratch: &std::path::Path) -> Arc<Identity> {
    Arc::new(Identity::on_fixture(
        connection.clone(),
        agent_reader_daemon::shipped_callers(),
        scratch.join("proc"),
    ))
}

fn block(xml: &str, name: &str) -> String {
    let start = xml
        .find(&format!("<interface name=\"{name}\">"))
        .unwrap_or_else(|| panic!("{name} missing"));
    let rest = &xml[start..];
    let end = rest.find("</interface>").expect("end") + "</interface>".len();
    rest[..end]
        .lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test]
async fn the_served_interface_is_the_declared_one_and_the_name_is_claimed_once() {
    let scratch = tempfile::tempdir().expect("scratch");
    let bus = PrivateBus::start(scratch.path());
    let first = bus.connect().await;
    serve_on(&first, service(), nobody(&first, scratch.path()))
        .await
        .expect("serves");
    let proxy = zbus::fdo::IntrospectableProxy::builder(&first)
        .destination(action_dbus::READER_BUS)
        .expect("destination")
        .path(action_dbus::READER_PATH)
        .expect("path")
        .build()
        .await
        .expect("proxy");
    let served = proxy.introspect().await.expect("introspection");
    assert_eq!(
        block(&served, "org.bardo.Reader1"),
        block(&introspection(Bus::Reader), "org.bardo.Reader1")
    );

    let second = bus.connect().await;
    let again = serve_on(&second, service(), nobody(&second, scratch.path())).await;
    assert!(matches!(again, Err(ServeError::Bus(_))), "{again:?}");
}
