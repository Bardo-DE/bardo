//! The scenes, by name: what each one maps and where the pointer rests.

use crate::scene::desk::{Desk, Dpi};
use compositor_core::geometry::{LogicalPoint, PhysicalSize};
use compositor_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use compositor_testclient::surface::lock::Locker;
use compositor_testclient::surface::popup::PopupSpec;
use compositor_testclient::{Pattern, shm_buffer};
use compositor_testkit::Fixture;

/// The app all the windows of a scene belong to.
const APP: &str = "org.example.scene";

/// Builds scene `name` on `fixture` and shoots its output.
pub fn scene(fixture: &mut Fixture, name: &str) -> image::RgbaImage {
    let mut desk = Desk::at(fixture);
    match name {
        "empty-desktop-1024x768" => desk.pointer_at(LogicalPoint { x: 512, y: 384 }),
        "one-window-1x" => one_window(&mut desk, Dpi::One),
        "one-window-1.5x" => one_window(&mut desk, Dpi::OneAndAHalf),
        "one-window-2x" => one_window(&mut desk, Dpi::Two),
        "cascaded-windows-1x" => cascaded(&mut desk),
        "popup-over-window-1x" => popup_over_window(&mut desk),
        "bar-and-window-1x" => bar_and_window(&mut desk),
        "lock-surface-1x" => locked(&mut desk),
        other => panic!("no scene named {other}"),
    }
    desk.shoot()
}

/// One 400x300 gradient window, drawn for `dpi`.
fn one_window(desk: &mut Desk<'_>, dpi: Dpi) {
    let top = desk.window(APP, (400, 300), Pattern::Gradient, dpi);
    desk.kept.keep(top);
}

/// Three windows of one app: the first centred, the rest cascaded.
fn cascaded(desk: &mut Desk<'_>) {
    let looks = [
        ((480, 320), Pattern::Checker { cell: 20 }),
        ((420, 300), Pattern::Gradient),
        ((360, 240), Pattern::CoordinateEncoding),
    ];
    for (size, pattern) in looks {
        let top = desk.window(APP, size, pattern, Dpi::One);
        desk.kept.keep(top);
    }
}

/// A menu hanging from a window.
fn popup_over_window(desk: &mut Desk<'_>) {
    let top = desk.window(APP, (600, 400), Pattern::Checker { cell: 25 }, Dpi::One);
    desk.popup(
        &top,
        PopupSpec::menu_at((40, 60)),
        Pattern::Gradient,
        Dpi::One,
    );
    desk.kept.keep(top);
}

/// A 28 px bar on the top edge that reserves its zone, and a window below it.
fn bar_and_window(desk: &mut Desk<'_>) {
    let bar = LayerAsk {
        level: Layer::Top,
        namespace: "bar",
        anchor: Anchor::Top | Anchor::Left | Anchor::Right,
        exclusive_zone: 28,
        keyboard: KeyboardInteractivity::None,
        size: (0, 28),
    };
    let layer = LayerSurface::create(desk.fixture.client(desk.id), &bar).expect("a layer surface");
    desk.fixture.pump();
    assert!(layer.ack(desk.fixture.client(desk.id)));
    let size = PhysicalSize { w: 1920, h: 28 };
    let buffer = shm_buffer(
        desk.fixture.client(desk.id),
        size,
        Pattern::Solid(0xff30_3a50),
    );
    layer.show(desk.fixture.client(desk.id), &buffer);
    desk.fixture.pump();
    let top = desk.window(APP, (800, 500), Pattern::Gradient, Dpi::One);
    desk.kept.keep((layer, buffer, top));
}

/// A window, then a session lock whose surface covers the output.
fn locked(desk: &mut Desk<'_>) {
    let top = desk.window(APP, (400, 300), Pattern::Gradient, Dpi::One);
    assert_eq!(
        desk.fixture.model().windows().len(),
        1,
        "a window is behind the lock"
    );
    let owner = desk.another();
    let client = desk.fixture.client(owner);
    let lock = Locker::request(client).expect("a lock");
    desk.fixture.pump();
    let cover = lock
        .cover(desk.fixture.client(owner))
        .expect("a lock surface");
    desk.fixture.pump();
    assert!(cover.ack(desk.fixture.client(owner)));
    let size = PhysicalSize { w: 1920, h: 1080 };
    let buffer = shm_buffer(
        desk.fixture.client(owner),
        size,
        Pattern::CoordinateEncoding,
    );
    cover.show(desk.fixture.client(owner), &buffer);
    desk.fixture.pump();
    assert!(lock.is_locked(), "the session is locked");
    desk.kept.keep((lock, cover, buffer));
    desk.kept.keep(top);
}
