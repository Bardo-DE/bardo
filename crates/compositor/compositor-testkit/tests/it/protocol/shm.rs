//! `shm` (P0-H1).

use compositor_core::geometry::PhysicalSize;
use compositor_testclient::surface::subsurface::{Compositor, H1Data, RawPool, seen};
use compositor_testclient::{ClientError, Pattern, shm_buffer};
use wayland_client::protocol::wl_shm::{Format, WlShm};

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

fn formats(fixture: &mut Fixture, id: ClientId, version: u32) -> Vec<String> {
    let shm: WlShm = fixture
        .client(id)
        .bind(version..=version, H1Data)
        .expect("wl_shm binds");
    fixture.pump();
    seen(fixture.client(id), &shm)
        .into_iter()
        .filter(|(event, _)| event == "format")
        .filter_map(|(_, args)| args.into_iter().next())
        .collect()
}

/// `Wl_shm` advertises argb8888 and xrgb8888.
#[test]
fn argb_and_xrgb_are_advertised() {
    let (mut fixture, id) = boot();
    for version in [1, 2] {
        // wl_shm.format codes: argb8888 is 0, xrgb8888 is 1.
        assert_eq!(
            formats(&mut fixture, id, version),
            ["0", "1"],
            "version {version}"
        );
    }
    assert!(matches!(
        fixture.client(id).bind::<WlShm, _>(3..=3, H1Data),
        Err(ClientError::MissingGlobal(_))
    ));
}

/// A buffer in a pool of the advertised format is accepted with no protocol error.
#[test]
fn a_buffer_is_accepted() {
    let (mut fixture, id) = boot();
    let _buffer = shm_buffer(
        fixture.client(id),
        PhysicalSize { w: 8, h: 8 },
        Pattern::Gradient,
    );
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
}

/// A buffer past its pool's end is `invalid_stride/invalid_fd` and disconnects only that client.
#[test]
fn a_pool_too_small_for_its_buffer_is_a_protocol_error() {
    let (mut fixture, bad) = boot();
    let good = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let client = fixture.client(bad);
    let pool = RawPool::new(client, 2, 64);
    let _buffer = pool.buffer(client, 0, (100, 100), 400, Format::Argb8888);
    fixture.pump();
    // wl_shm.error.invalid_stride, posted on the pool.
    assert_eq!(fault(&mut fixture, bad), ("wl_shm_pool".to_owned(), 1));
    assert!(fixture.client(good).is_connected());
}

/// A format the compositor did not advertise is `wl_shm.invalid_format` (0).
#[test]
fn an_unadvertised_format_is_a_protocol_error() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let pool = RawPool::new(client, 2, 4096);
    let _buffer = pool.buffer(client, 0, (8, 8), 16, Format::Rgb565);
    fixture.pump();
    assert_eq!(fault(&mut fixture, id), ("wl_shm_pool".to_owned(), 0));
}

/// An empty pool is `wl_shm.invalid_stride` (1), posted on the `wl_shm`.
#[test]
fn an_empty_pool_is_a_protocol_error() {
    let (mut fixture, id) = boot();
    let _pool = RawPool::new(fixture.client(id), 2, 0);
    fixture.pump();
    assert_eq!(fault(&mut fixture, id), ("wl_shm".to_owned(), 1));
}

/// Destroying a pool leaves its buffers valid (the mapping lives while a buffer does).
#[test]
fn a_buffer_outlives_its_pool() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let size = PhysicalSize { w: 64, h: 32 };
    let pool = RawPool::new(client, 2, 64 * 32 * 4);
    pool.write(0, &Pattern::Gradient.bytes(size));
    let buffer = pool.buffer(client, 0, (64, 32), 256, Format::Argb8888);
    pool.pool.destroy();
    let surface = Compositor::bind(client, 5).surface(client);
    surface.attach_raw(&buffer);
    surface.commit();
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
}
