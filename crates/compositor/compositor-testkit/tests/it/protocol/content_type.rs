//! `content_type` (P0-H1).

use compositor_testclient::misc::content_type::{ContentTypes, Type, set};
use compositor_testclient::surface::subsurface::Compositor;

use compositor_core::policy::ConnectionRole;
use compositor_testkit::{ClientId, Fixture, FixtureSpec};

fn boot() -> (Fixture, ClientId) {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    (fixture, id)
}

/// The (interface, code) of the protocol error the compositor killed `id` with.
fn fault(fixture: &mut Fixture, id: ClientId) -> (String, u32) {
    let error = fixture
        .client(id)
        .protocol_error()
        .expect("the compositor posted an error");
    (error.interface, error.code)
}

/// `Set_content_type(video)` is read at commit.
///
/// What the compositor stores is not visible through the fixture (it holds the model and the
/// frame, not the protocol parts), so this checks the protocol side: every value is accepted
/// at commit, in any order, with no error; `compositor_wayland::content_type`'s unit tests check
/// the stored kind.
#[test]
fn the_content_type_is_stored() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let object = ContentTypes::bind(client).for_surface(client, &surface);
    for kind in [Type::Video, Type::Game, Type::Photo, Type::None] {
        set(&object, kind);
        surface.commit();
    }
    object.destroy();
    surface.commit();
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
}

/// A second content-type object on one surface is
/// `wp_content_type_manager_v1.already_constructed` (0).
#[test]
fn a_second_content_type_object_is_a_protocol_error() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let manager = ContentTypes::bind(client);
    let _first = manager.for_surface(client, &surface);
    let _second = manager.for_surface(client, &surface);
    fixture.pump();
    assert_eq!(
        fault(&mut fixture, id),
        ("wp_content_type_manager_v1".to_owned(), 0)
    );
}
