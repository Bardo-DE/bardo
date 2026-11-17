//! The base scenes (and the vkms one) (P0-R-scene, on pixman), driven through the fixture so
//! real client surfaces are drawn.

use compositor_testkit::{Fixture, FixtureSpec};

/// The P0 base scenes: name, outputs.
const SCENES: [(&str, &[&str]); 8] = [
    // The vkms writeback scene (dev/vkms/accept.sh): the default 1024x768 mode of a vkms CRTC.
    (
        "empty-desktop-1024x768",
        &["HEADLESS-1:1024x768@60000:120:0,0"],
    ),
    ("one-window-1x", &["HEADLESS-1:1920x1080@60000:120:0,0"]),
    ("one-window-1.5x", &["HEADLESS-1:2880x1620@60000:180:0,0"]),
    ("one-window-2x", &["HEADLESS-1:3840x2160@60000:240:0,0"]),
    (
        "cascaded-windows-1x",
        &["HEADLESS-1:1920x1080@60000:120:0,0"],
    ),
    (
        "popup-over-window-1x",
        &["HEADLESS-1:1920x1080@60000:120:0,0"],
    ),
    ("bar-and-window-1x", &["HEADLESS-1:1920x1080@60000:120:0,0"]),
    ("lock-surface-1x", &["HEADLESS-1:1920x1080@60000:120:0,0"]),
];

mod build;
mod client_round;
mod clip;
mod desk;
mod frames;
mod minimise;
mod mission;

/// Builds scene `name` on `fixture` with test clients and shoots its output.
fn shoot(fixture: &mut Fixture, name: &str) -> image::RgbaImage {
    build::scene(fixture, name)
}

/// The physical size a scene's output string (`NAME:WxH@RATE:SCALE:X,Y`) asks for.
fn mode_of(output: &str) -> (u32, u32) {
    let mode = output.split(':').nth(1).expect("a mode");
    let (size, _) = mode.split_once('@').expect("a refresh rate");
    let (w, h) = size.split_once('x').expect("a width and a height");
    (w.parse().expect("a width"), h.parse().expect("a height"))
}

#[test]
fn the_base_scenes_are_the_size_of_their_output_and_draw_their_clients() {
    let failures: Vec<String> = SCENES
        .iter()
        .filter_map(|(name, outputs)| {
            let mut fixture =
                Fixture::start(FixtureSpec::desktop().with_outputs(outputs)).expect(name);
            let shot = shoot(&mut fixture, name);
            if (shot.width(), shot.height()) != mode_of(outputs[0]) {
                return Some(format!(
                    "{name}: shot is {}x{}",
                    shot.width(),
                    shot.height()
                ));
            }
            let first = *shot.get_pixel(0, 0);
            let flat = shot.pixels().all(|px| *px == first);
            let windows = name.starts_with("one-window")
                || name.starts_with("cascaded")
                || name.starts_with("popup")
                || name.starts_with("bar-and");
            (windows && flat).then(|| format!("{name}: no client is drawn"))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
