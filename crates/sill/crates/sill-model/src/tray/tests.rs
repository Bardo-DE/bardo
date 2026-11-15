use super::{
    Check, TRAY_ITEMS_MAX, Tray, TrayEvent, TrayIcon, TrayItem, TrayItemId, TrayMenuEntry,
    TrayMenuIcon, TrayMenuKind, TrayStatus, step,
};
use crate::units::Visibility;

fn id(name: &str) -> TrayItemId {
    TrayItemId(format!(":1.{name}/StatusNotifierItem"))
}

fn item(name: &str) -> TrayItem {
    TrayItem {
        id: id(name),
        title: name.to_owned(),
        icon: TrayIcon::Named(format!("{name}-symbolic")),
        status: TrayStatus::Active,
        menu: Vec::new(),
    }
}

fn tray(names: &[&str]) -> Tray {
    Tray {
        items: names.iter().map(|n| item(n)).collect(),
    }
}

fn quit_menu() -> Vec<TrayMenuEntry> {
    vec![TrayMenuEntry {
        id: 7,
        kind: TrayMenuKind::Item {
            label: "Quit".to_owned(),
            enabled: true,
            visibility: Visibility::Shown,
            check: Check::None,
            icon: TrayMenuIcon::None,
            children: Vec::new(),
            shortcut: None,
            command: None,
        },
    }]
}

#[test]
fn events_reduce_to_the_expected_tray() {
    let with = |name: &str, edit: fn(&mut TrayItem)| {
        let mut t = tray(&["a", "b"]);
        if let Some(i) = t.items.iter_mut().find(|i| i.title == name) {
            edit(i);
        }
        t
    };
    let cases: Vec<(&str, Tray, TrayEvent, Tray)> = vec![
        (
            "registration appends",
            tray(&["a"]),
            TrayEvent::Registered(item("b")),
            tray(&["a", "b"]),
        ),
        (
            "re-registration replaces in place",
            tray(&["a", "b"]),
            TrayEvent::Registered(TrayItem {
                status: TrayStatus::NeedsAttention,
                ..item("a")
            }),
            with("a", |i| i.status = TrayStatus::NeedsAttention),
        ),
        (
            "title",
            tray(&["a", "b"]),
            TrayEvent::Title {
                item: id("b"),
                title: "B!".to_owned(),
            },
            {
                let mut t = tray(&["a", "b"]);
                t.items[1].title = "B!".to_owned();
                t
            },
        ),
        (
            "icon",
            tray(&["a", "b"]),
            TrayEvent::Icon {
                item: id("a"),
                icon: TrayIcon::None,
            },
            with("a", |i| i.icon = TrayIcon::None),
        ),
        (
            "status",
            tray(&["a", "b"]),
            TrayEvent::Status {
                item: id("b"),
                status: TrayStatus::Passive,
            },
            with("b", |i| i.status = TrayStatus::Passive),
        ),
        (
            "menu",
            tray(&["a", "b"]),
            TrayEvent::Menu {
                item: id("a"),
                menu: quit_menu(),
            },
            with("a", |i| i.menu = quit_menu()),
        ),
        (
            "removal",
            tray(&["a", "b"]),
            TrayEvent::Removed(id("a")),
            tray(&["b"]),
        ),
        (
            "a change for an unknown item is dropped",
            tray(&["a"]),
            TrayEvent::Title {
                item: id("zz"),
                title: "x".to_owned(),
            },
            tray(&["a"]),
        ),
        (
            "removing an unknown item changes nothing",
            tray(&["a"]),
            TrayEvent::Removed(id("zz")),
            tray(&["a"]),
        ),
    ];
    for (name, before, event, want) in cases {
        assert_eq!(step(before, event), want, "{name}");
    }
}

#[test]
fn a_program_cannot_register_the_tray_full() {
    let crowd: Vec<String> = (0..TRAY_ITEMS_MAX + 20).map(|n| n.to_string()).collect();
    let names: Vec<&str> = crowd.iter().map(String::as_str).collect();
    let full = names.iter().fold(Tray::default(), |tray, name| {
        step(tray, TrayEvent::Registered(item(name)))
    });
    assert_eq!(full.items.len(), TRAY_ITEMS_MAX);
    let first = item(names[0]);
    let again = step(full.clone(), TrayEvent::Registered(first));
    assert_eq!(again, full, "a re-registration still replaces in place");
    let later = step(full.clone(), TrayEvent::Registered(item("late")));
    assert_eq!(later, full, "a new one is turned away");
    let room = step(full, TrayEvent::Removed(id(names[0])));
    let made = step(room, TrayEvent::Registered(item("late")));
    assert_eq!(made.items.len(), TRAY_ITEMS_MAX, "room freed, room taken");
}
