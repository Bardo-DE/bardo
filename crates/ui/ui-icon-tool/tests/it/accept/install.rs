//! `icons install` run as a user runs it: the repository's set lands under
//! `--to`, every app in every style and size, and `ui_settings`'s lookup finds a file there.

use base_testkit::TestEnv;
use settings_types::Px;
use std::path::Path;
use ui_components::style::icon::retint::IconStyle;
use ui_settings::icon_assets::{APP_ICON_PX, AppIconName, Presence, find_app_icon};

#[test]
fn install_copies_the_set_where_the_lookup_finds_it() {
    let env = TestEnv::new("icons-install");
    // Nothing is there on a first run.
    let to = env.data().join("bardo/icons/apps");
    let status = env
        .command(env!("CARGO_BIN_EXE_icons"))
        .args(["install", "--to"])
        .arg(&to)
        .status()
        .expect("icons runs");
    assert!(status.success());
    let present = |path: &Path| {
        if path.is_file() {
            Presence::Present
        } else {
            Presence::Absent
        }
    };
    for app in ["mail", "files", "terminal", "notes", "photos", "viewer"] {
        let name = AppIconName::parse(app).expect("a name");
        for style in [IconStyle::Colour, IconStyle::Muted, IconStyle::Monochrome] {
            for px in APP_ICON_PX {
                let got = find_app_icon(&to, &name, Px(px), style, present);
                assert!(
                    got.is_some_and(|path| path.ends_with(format!("{px}.png"))),
                    "{app} {style:?} {px}"
                );
            }
        }
    }
}
