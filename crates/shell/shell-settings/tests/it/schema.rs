//! design/22-SETTINGS.md §9.6 for bardo-shell's schema: it round-trips, every kind has a widget, it
//! names bardo-shell's file, the Settings pages match §5, and the deep links resolve.

use settings_types::schema::{Exposure, Page, Schema, deep_link_path};
use shell_settings::{schema, schemas};

#[test]
fn schema_round_trip() {
    for schema in schemas().into_iter().chain([schema()]) {
        let text = schema.to_toml();
        let back = Schema::from_toml(&text).unwrap_or_else(|e| panic!("{text}: {e}"));
        assert_eq!(back, schema, "{text}");
    }
}

#[test]
fn every_domain_names_sills_file() {
    for schema in schemas().into_iter().chain([schema()]) {
        assert_eq!(schema.app.0, "shell");
        assert_eq!(schema.file.0, "shell/settings.toml");
        assert_eq!(schema.version, 1);
    }
}

/// §5: the keys each Settings page shows. Every other bardo-shell key is Advanced (file only).
const BASIC: &[&str] = &[
    "bar.autohide",
    "dock.magnification",
    "dock.tile_size_px",
    "dock.autohide",
    "dock.indicators",
    "dock.trash",
    "scroll.natural",
    "notifications.dnd",
    "notifications.banner_style",
    "companion.summon",
    "companion.in_field",
    "companion.notify",
    "voice.hold_to_talk",
    "wallpaper.picture",
    "hot_corners.top_left",
    "hot_corners.top_right",
    "hot_corners.bottom_left",
    "hot_corners.bottom_right",
];

#[test]
fn exposure_follows_section_5() {
    for key in schema().key {
        let want = if BASIC.contains(&key.path.0.as_str()) {
            Exposure::Basic
        } else {
            Exposure::Advanced
        };
        assert_eq!(key.exposure, want, "{}", key.path.0);
    }
}

#[test]
fn basic_keys_are_on_their_section_5_pages() {
    let page_of = |prefix: &str| match prefix {
        "bar" => Page::Appearance,
        "dock" | "hot_corners" => Page::Dock,
        "scroll" => Page::MouseAndGestures,
        "notifications" | "sound" => Page::Notifications,
        "spaces" => Page::Spaces,
        "companion" | "voice" => Page::Intelligence,
        "wallpaper" => Page::declared("wallpaper"),
        other => panic!("no §5 page for {other}"),
    };
    for key in schema().key {
        if key.exposure == Exposure::Basic {
            let domain = key.path.0.split('.').next().unwrap_or_default().to_owned();
            assert_eq!(key.page, page_of(&domain), "{}", key.path.0);
        }
    }
}

/// §5: the Power page holds bardo-shell's idle service, every `idle.*` key and the lock grace,
/// all Advanced; nothing else is on it, and the rest of `session.*` stays on Accounts.
#[test]
fn the_power_page_is_idle_and_the_lock_grace() {
    let on_power = |path: &str| path.starts_with("idle.") || path == "session.lock_grace_s";
    let whole = schema();
    for key in &whole.key {
        let path = key.path.0.as_str();
        match (on_power(path), &key.page) {
            (true, Page::Power) => assert_eq!(key.exposure, Exposure::Advanced, "{path}"),
            (true, other) => panic!("{path} is on {other:?}, not Power"),
            (false, Page::Power) => panic!("{path} is on Power"),
            (false, _) => {}
        }
        if path.starts_with("session.") && !on_power(path) {
            assert_eq!(key.page, Page::Accounts, "{path}");
        }
    }
    assert_eq!(whole.key.iter().filter(|k| on_power(&k.path.0)).count(), 6);
}

/// The idle times carry their unit (`Secs` "s", `Mins` "min") and their §3.24 range.
#[test]
fn idle_times_are_bounded_in_their_units() {
    use settings_types::schema::KeyKind;
    const CASES: &[(&str, i64, &str)] = &[
        ("session.lock_grace_s", 60, "s"),
        ("idle.screen_off_ac_min", 240, "min"),
        ("idle.screen_off_battery_min", 240, "min"),
        ("idle.suspend_ac_min", 480, "min"),
        ("idle.suspend_battery_min", 480, "min"),
    ];
    let whole = schema();
    for &(path, max, unit) in CASES {
        let key = whole
            .key
            .iter()
            .find(|k| k.path.0 == path)
            .unwrap_or_else(|| panic!("{path} missing"));
        assert_eq!(
            key.kind,
            KeyKind::Bounded {
                min: 0,
                max,
                unit: Some(unit.to_owned())
            },
            "{path}"
        );
    }
}

#[test]
fn deep_links_resolve_through_the_schemas() {
    const CASES: &[(&str, &str)] = &[
        ("magnification", "dock.magnification"),
        ("natural scrolling", "scroll.natural"),
        ("do not disturb", "notifications.dnd"),
        ("automatically hide the dock", "dock.autohide"),
        ("hide and show the menu bar", "bar.autohide"),
    ];

    let all = schemas();
    for &(query, want) in CASES {
        assert_eq!(
            deep_link_path(&all, query).map(|p| p.0),
            Some(want.to_owned()),
            "{query:?}"
        );
    }
}

/// `bar.autohide` offers the Mac's four choices in its order, by the words the file holds and
/// the labels System Settings shows; the Mac's default is the full-screen one.
#[test]
fn the_menu_bar_choices_are_the_macs() {
    use settings_types::schema::KeyKind;
    use shell_settings::MenuBarHiding;
    use ui_core::word::Word;
    const WORDS: [(MenuBarHiding, &str, &str); 4] = [
        (MenuBarHiding::Always, "always", "Always"),
        (
            MenuBarHiding::DesktopOnly,
            "desktop_only",
            "On Desktop Only",
        ),
        (
            MenuBarHiding::FullScreenOnly,
            "full_screen_only",
            "In Full Screen Only",
        ),
        (MenuBarHiding::Never, "never", "Never"),
    ];
    for (choice, slug, label) in WORDS {
        assert_eq!((choice.slug(), choice.label()), (slug, label));
    }
    assert_eq!(MenuBarHiding::default(), MenuBarHiding::FullScreenOnly);
    let whole = schema();
    let key = whole
        .key
        .iter()
        .find(|k| k.path.0 == "bar.autohide")
        .unwrap_or_else(|| panic!("bar.autohide missing"));
    assert_eq!(key.label.0, "Automatically hide and show the menu bar");
    assert_eq!(key.page, Page::Appearance);
    assert_eq!(
        key.kind,
        KeyKind::Segmented {
            variants: WORDS
                .iter()
                .map(|(_, slug, _)| (*slug).to_owned())
                .collect()
        }
    );
}
