//! The catcher closes the launcher on a press anywhere on it (design/06 §20.2), in a
//! `ui_harness::Harness`.

use crate::launcher_harness;

use ui_components::prelude::*;
use std::time::Duration;

use ui_harness::{Input, Viewport};
use launcher_harness::launcher_support::fake;
use shell_launcher_ui::LauncherCatcher;
use shell_ui_kit::harness::SurfaceHarness;
use shell_ui_kit::links::LauncherLink;

#[test]
fn a_press_on_the_catcher_closes_the_launcher() {
    let link = LauncherLink::new();
    link.toggle(fake::output().id);
    let mut catcher = SurfaceHarness::<LauncherCatcher>::setup(link.clone(), ())
        .viewport(Viewport {
            width: 960,
            height: 540,
            scale_percent: 100,
        })
        .start();
    catcher.harness.advance(Duration::from_millis(20));
    assert!(link.now().opening().is_some());
    catcher.harness.send(Input::click(Point {
        x: Px(20.0),
        y: Px(500.0),
    }));
    catcher.harness.advance(Duration::from_millis(20));
    assert!(link.now().opening().is_none(), "{}", catcher.harness.html());
}
