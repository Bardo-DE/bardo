//! The two launcher crashes, reproduced on a real Blitz document.
//!
//! A motion hook's settle task outlived its component, so unmounting a palette or a menu
//! before its entrance settled set a dropped signal ("`ValueDroppedError`"). A field's
//! `Focus::OnMount` set the focus from a task polled inside the render pass, where the renderer
//! holds the document ("`RefCell` already borrowed"). Each test drives mounts and unmounts from a
//! script inside the page, on the harness's clock, and passes only if nothing panics.

use dioxus::prelude::*;
use std::time::Duration;
use ui_components::base::time::clock::sleep;
use ui_components::components::menus::palette::palette_group::{PaletteGroup, PaletteRow};
use ui_components::host::measure::Anchor;
use ui_components::prelude::*;
use ui_harness::{Clock, Harness, HarnessConfig, Viewport};

const VIEW: Viewport = Viewport {
    width: 640,
    height: 420,
    scale_percent: 100,
};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// The titles both subjects list.
const TITLES: [&str; 3] = ["Open", "Pause", "Quit"];

/// What the script mounts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Subject {
    Palette,
    Menu,
}

/// One scripted step: wait, then show or hide the subject.
type Step = (u64, Shown);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shown {
    Yes,
    No,
}

/// A tiny component, remounted every tick: it keeps dioxus reusing freed scope slots, as the
/// launcher's result rows do. A task that outlived the palette is polled again only once its
/// old slot holds a live scope, and then it wrote to the palette's dropped signals.
#[component]
fn Churn(n: u32) -> Element {
    rsx! { i { "{n}" } }
}

/// A page that mounts `subject` and unmounts it on `script`'s schedule, counting mounts, while
/// the palette's results change under it every 16 ms (a re-render in the same turn as the
/// mount, which is when that crash fired most) and small components come and go beside it.
#[component]
fn Scripted(subject: Subject, script: Vec<Step>, motion: Motion) -> Element {
    let mut shown = use_signal(|| Shown::No);
    let mut mounts = use_signal(|| 0u32);
    let mut tick = use_signal(|| 0u32);
    use_hook(move || {
        spawn(async move {
            for (wait, next) in script {
                sleep(ms(wait)).await;
                if next == Shown::Yes && *shown.peek() == Shown::No {
                    mounts += 1;
                }
                shown.set(next);
            }
        });
        spawn(async move {
            loop {
                sleep(ms(16)).await;
                tick += 1;
            }
        });
    });
    let count = tick() % 3 + 1;
    let taken = TITLES.iter().zip(1u8..).take(count as usize);
    let rows: Vec<MenuItem<u8>> = taken
        .clone()
        .map(|(title, value)| MenuItem::new(value, *title))
        .collect();
    let palette_rows: Vec<PaletteRow<u8>> = taken
        .map(|(title, value)| PaletteRow::new(value, *title))
        .collect();
    rsx! {
        Ds { appearance: Appearance { motion, ..Appearance::default() }, material: Material::Window,
            p { class: "mounts", "{mounts}" }
            for n in 0..count {
                Churn { key: "{tick}-{n}", n }
            }
            if shown() == Shown::Yes {
                match subject {
                    Subject::Palette => rsx! {
                        CommandPalette::<u8> {
                            label: "Search".to_string(),
                            placeholder: "Search".to_string(),
                            query: String::new(),
                            tokens: Vec::new(),
                            groups: vec![PaletteGroup::list("Results", palette_rows)],
                            empty: "Nothing".to_string(),
                            oninput: move |_| {},
                            onpick: move |_| {},
                            onclose: move |_| shown.set(Shown::No),
                        }
                    },
                    Subject::Menu => rsx! {
                        Menu::<u8> {
                            placement: MenuPlacement::Popup,
                            anchor: Anchor::Point(Point { x: Px(40.0), y: Px(60.0) }),
                            items: rows,
                            onpick: move |_| {},
                            onclose: move |()| shown.set(Shown::No),
                        }
                    },
                }
            }
        }
    }
}

/// Mount at 10 ms, unmount 100 ms later: before the palette's entrance (`peek-in`) settles.
fn early_unmount() -> Vec<Step> {
    vec![(10, Shown::Yes), (100, Shown::No)]
}

#[expect(non_snake_case, reason = "a Dioxus component function is named in CamelCase")]
fn PaletteGoneEarly() -> Element {
    rsx! { Scripted { subject: Subject::Palette, script: early_unmount(), motion: Motion::Standard } }
}

#[expect(non_snake_case, reason = "a Dioxus component function is named in CamelCase")]
fn MenuGoneEarly() -> Element {
    rsx! { Scripted { subject: Subject::Menu, script: early_unmount(), motion: Motion::Standard } }
}

fn mounts(harness: &Harness) -> String {
    harness.text_of(".mounts").unwrap_or_default()
}

/// Let the script run for `time`, then on until it has mounted `want` times and the subject is
/// gone, for at most as long again: a loaded machine stretches every wait the script makes.
fn run_script(harness: &mut Harness, time: Duration, want: &str, subject: &str) {
    harness.advance(time);
    for _ in 0..(time.as_millis() / 100) {
        if mounts(harness) == want && harness.count(subject) == 0 {
            return;
        }
        harness.advance(ms(100));
    }
}

/// How long `anim`'s entrance runs, asked of the motion table: the early unmount is inside it.
fn entrance(anim: Anim) -> Duration {
    settle(anim, MotionLevel::Standard)
}

/// The page on the virtual clock: the script's sleeps, the motion timers and the CSS all read
/// one clock that `advance` moves, so nothing here depends on how loaded the machine is (on the
/// wall clock a 16 ms tick against slow rounds made the run late and the test flaky). A test
/// still reads what the script did from the page's own count rather than catching the palette
/// mid-way.
fn virtual_harness(app: fn() -> Element) -> Harness {
    Harness::new(app, HarnessConfig::new(VIEW).with_clock(Clock::Virtual))
}

#[test]
fn a_palette_unmounted_before_its_entrance_settles_does_not_panic() {
    assert!(entrance(Anim::PeekIn) > ms(110), "the unmount is early");
    let mut harness = virtual_harness(PaletteGoneEarly);
    run_script(
        &mut harness,
        entrance(Anim::PeekIn) + ms(400),
        "1",
        ".ds-palette",
    );
    assert_eq!(mounts(&harness), "1", "the palette mounted");
    assert_eq!(harness.count(".ds-palette"), 0, "and is gone");
}

#[test]
fn a_menu_unmounted_before_its_fade_settles_does_not_panic() {
    let mut harness = virtual_harness(MenuGoneEarly);
    run_script(
        &mut harness,
        settle(Anim::MenuOut, MotionLevel::Standard) + ms(400),
        "1",
        ".ds-menu",
    );
    assert_eq!(mounts(&harness), "1", "the menu mounted");
    assert_eq!(harness.count(".ds-menu"), 0, "and is gone");
}
