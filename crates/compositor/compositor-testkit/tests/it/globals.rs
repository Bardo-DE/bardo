//! The globals a public client is shown by the default fixture: the core ones, each once, and
//! none of the shell's own. Which power gates which global is `compositor_core`'s table, tested
//! beside it and through the consent tests of the protocol suite.

use compositor_core::policy::ConnectionRole;
use compositor_testkit::{Fixture, FixtureSpec};
use std::collections::BTreeSet;

/// The interfaces `role` is advertised by the default fixture, sorted.
fn advertised_to(role: ConnectionRole) -> Vec<String> {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(role);
    // A sorted list, not a set: a global advertised twice must not hide.
    let mut interfaces: Vec<String> = fixture
        .client(client)
        .globals()
        .contents()
        .clone_list()
        .into_iter()
        .map(|global| global.interface)
        .collect();
    interfaces.sort();
    interfaces
}

/// A public client finds what any toolkit needs, and finds each global once (`wl_output` once
/// for the one output).
#[test]
fn the_default_fixture_advertises_the_core_globals_once_to_a_public_client() {
    let shown = advertised_to(ConnectionRole::Public);
    let unique: BTreeSet<&String> = shown.iter().collect();
    assert_eq!(
        unique.len(),
        shown.len(),
        "a global is advertised twice: {shown:?}"
    );
    for core in [
        "wl_compositor",
        "wl_subcompositor",
        "wl_shm",
        "wl_output",
        "wl_seat",
        "xdg_wm_base",
        "wp_viewporter",
        "wp_fractional_scale_manager_v1",
    ] {
        assert!(shown.iter().any(|interface| interface == core), "{core}");
    }
}

/// The shell's own protocols are for the shell: a public client never sees them, and the shell
/// sees everything the public socket does besides.
#[test]
fn the_shell_globals_are_only_for_the_shell() {
    let public = advertised_to(ConnectionRole::Public);
    let shell = advertised_to(ConnectionRole::Shell);
    for own in ["bardo_shell_v1", "bardo_desktop_manager_v1"] {
        assert!(!public.iter().any(|interface| interface == own), "{own}");
        assert!(shell.iter().any(|interface| interface == own), "{own}");
    }
    for interface in &public {
        assert!(
            shell.contains(interface),
            "{interface} is hidden from the shell"
        );
    }
}
