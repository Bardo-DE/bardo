//! A search card summoned by a key and typed into at once (Spotlight, mail's ⌘K): the field is
//! asked for by selector with its text selected, and every key typed from the frame the field
//! first reads focused lands in order. A caret moved after the first key once turned "invoice"
//! into "nvoicei".

use dioxus::prelude::*;
use std::time::Duration;
use ui_components::focus::select::Select;
use ui_components::focus::selector::focus_by_selector;
use ui_components::prelude::*;
use ui_harness::{Clock, Harness, HarnessConfig, Input, Viewport};

const VIEW: Viewport = Viewport {
    width: 800,
    height: 520,
    scale_percent: 100,
};

const FIELD: &str = ".ds-search-card input";

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// The rows mail shows: its recents before the first key, then results for the query, which
/// arrive a moment later and change in number as they do.
fn sections(query: &str, drawn: u32) -> Vec<SuggestionSection<String>> {
    if query.is_empty() {
        return vec![SuggestionSection::titled(
            "Recents",
            vec![MenuItem::new("Compose".to_owned(), "Compose")],
        )];
    }
    let rows = (0..=drawn)
        .map(|n| MenuItem::new(format!("{query} {n}"), format!("{query} {n}")))
        .collect();
    vec![SuggestionSection::titled("Mail", rows)]
}

/// A button that summons the card as mail's ⌘K does: the query cleared, the card shown, and the
/// field asked for by selector with its text selected.
#[expect(
    non_snake_case,
    reason = "a Dioxus component function is named in CamelCase"
)]
fn Page() -> Element {
    let mut query = use_signal(String::new);
    let mut shown = use_signal(|| false);
    let mut in_a_field = use_signal(|| false);
    let mut drawn = use_signal(|| 0u32);
    // Results land a few milliseconds after each key, as a search does: another render of the
    // card while keys are still coming.
    use_effect(move || {
        let _ = query();
        spawn(async move {
            ui_components::base::time::clock::sleep(Duration::from_millis(3)).await;
            drawn.with_mut(|n| *n += 1);
        });
    });
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window,
            div { style: "position:relative; height:520px",
                Button { label: "Summon",
                    onclick: move |_| {
                        query.set(String::new());
                        shown.set(true);
                        spawn(async move {
                            let _ = focus_by_selector(FIELD, Select::All).await;
                        });
                    }
                }
                if shown() {
                    div { style: "position:absolute; left:40px; top:60px; width:400px",
                        SearchField::<String> {
                            label: "Search",
                            value: query(),
                            suggestions: sections(&query(), drawn()),
                            open: Panel::Shown,
                            stop_at_ends: true,
                            present: SuggestionsPresent::Card,
                            oninput: move |next: String| query.set(next),
                            onfocus: move |()| in_a_field.set(true),
                            onpick: move |_: String| {},
                        }
                    }
                }
                p { id: "value", style: "position:absolute; left:500px; top:10px", "{query}" }
                p { id: "in-field", style: "position:absolute; left:500px; top:30px", "{in_a_field}" }
            }
        }
    }
}

/// Click Summon, then step frame by frame until the field reads focused.
fn summoned() -> Harness {
    let mut harness = Harness::new(Page, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.advance(ms(100));
    let at = harness.centre(".ds-button").expect("the summon button");
    harness.send(Input::click(at));
    let mut frames = 0;
    while !harness.focus_of(FIELD) {
        if !harness.step_frame() {
            harness.advance(ms(1));
        }
        frames += 1;
        assert!(frames < 120, "the field never took the keyboard");
    }
    harness
}

fn typed(harness: &mut Harness) -> String {
    harness.advance(ms(300));
    harness.text_of("#value").unwrap_or_default()
}

#[test]
fn keys_typed_after_the_focus_land_in_order_at_any_pace() {
    // How each key is followed: at once, a frame later, a few milliseconds later.
    let paces: [(&str, fn(&mut Harness)); 3] = [
        ("at once", |_| {}),
        ("a frame apart", |harness| {
            harness.step_frame();
        }),
        ("a few milliseconds apart", |harness| {
            harness.advance(ms(4));
        }),
    ];
    for (pace, after_each) in paces {
        let mut harness = summoned();
        "invoice".chars().for_each(|c| {
            harness.send(Input::key(ShortcutKey::Char(c)));
            after_each(&mut harness);
        });
        assert_eq!(typed(&mut harness), "invoice", "{pace}");
    }
}

#[test]
fn the_field_is_the_same_node_after_the_first_key_and_the_results() {
    let mut harness = summoned();
    let before = harness.attr(FIELD, "data-dioxus-id");
    assert!(before.is_some(), "the field carries its node id");
    harness.send(Input::key(ShortcutKey::Char('i')));
    harness.advance(ms(40));
    assert_eq!(
        harness.attr(FIELD, "data-dioxus-id"),
        before,
        "{}",
        harness.html()
    );
}
