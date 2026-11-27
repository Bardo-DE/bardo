//! A Bardo window on the portable Blitz shell: a `Ds` root with a Button that opens a Menu and
//! raises the undo Toast. It closes itself after three seconds, or on Escape.
//!
//! `cargo run -p ui-blitz --example window`

use base_machine::time::clock::sleep;
use dioxus::prelude::*;
use dioxus_native::use_window_event;
use dioxus_native::winit::event::{ElementState, WindowEvent};
use dioxus_native::winit::keyboard::Key;
use std::time::Duration;
use ui_blitz::{AppConfig, AppId, WindowSize, launch};
use ui_components::components::controls::button_model::Answers;
use ui_components::host::measure::Anchor;
use ui_components::prelude::*;
use ui_components::stack::toast_hub::use_toast_hub;

/// How long the window stays up on its own.
const LIFETIME: Duration = Duration::from_secs(3);

fn main() -> Result<(), ui_blitz::LaunchError> {
    launch(
        App,
        AppConfig::new("bardo: ui-blitz window", WindowSize::new(480, 320))
            .with_app_id(AppId("dev.bardo.Window".to_owned())),
    )
}

#[expect(
    non_snake_case,
    reason = "a Dioxus component function is named in CamelCase"
)]
fn App() -> Element {
    use_hook(|| {
        spawn(async {
            sleep(LIFETIME).await;
            std::process::exit(0);
        })
    });
    use_window_event(|event, event_loop| {
        if let WindowEvent::KeyboardInput { event, .. } = event
            && event.state == ElementState::Pressed
            && event.logical_key == Key::Named(NamedKey::Escape)
        {
            event_loop.exit();
        }
    });
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window, Demo {} }
    }
}

/// The controls, inside the root so they can reach its toast hub.
#[expect(
    non_snake_case,
    reason = "a Dioxus component function is named in CamelCase"
)]
fn Demo() -> Element {
    let mut open = use_signal(|| Check::Off);
    let toasts = use_toast_hub();
    rsx! {
        Button {
            answers: Answers::Return,
            label: "Snooze…",
            value: Some(open()),
            onclick: move |_| {
                open.set(Check::On);
                toasts.push("Snoozed until tomorrow".into(), None);
            },
        }
        if open() == Check::On {
            Menu {
                placement: MenuPlacement::Popup,
                anchor: Anchor::Point(Point { x: Px(24.0), y: Px(72.0) }),
                items: entries(),
                onpick: move |_: u8| open.set(Check::Off),
                onclose: move |()| open.set(Check::Off),
            }
        }
    }
}

fn entries() -> Vec<MenuItem<u8>> {
    ["Later today", "Tomorrow", "Next week"]
        .into_iter()
        .zip(0..)
        .map(|(title, value)| MenuItem::Item {
            value,
            title: title.into(),
            image: None,
            key: None,
            check: None,
            availability: Availability::Enabled,
            hint: None,
            keep_open: false,
            text: ItemText::default(),
        })
        .collect()
}
