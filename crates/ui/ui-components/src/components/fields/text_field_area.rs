//! The `textarea` a multi-line `TextField` draws: the same placeholder span over it as a line,
//! and Enter is a newline, never a commit. The caret leaving commits, as it does for a line.

use crate::components::fields::text_field_parts::{Field, aria_placeholder, placeholder_shown};
use dioxus::prelude::*;
use ui_core::vocab::Availability;
use ui_core::word::Word;

/// Several lines of text, as tall as the field's `rows`.
pub(crate) fn area(field: Field, value: &str) -> Element {
    let Field {
        bezeled,
        rows,
        size,
        label,
        id,
        placeholder,
        availability,
        invalid,
        description,
        focus,
        focuser,
        caret,
        handlers,
        told,
        ime,
        blink,
    } = field;
    let shown = placeholder_shown(value, &placeholder, ime.composing());
    rsx! {
        span {
            class: "ds-input-wrap",
            "data-variant": if bezeled { "bezeled" } else { "plain" },
            "data-size": size.slug(),
            "data-kind": "multiline",
            "data-rows": rows.slug(),
            "data-blink": blink.attr(),
            textarea {
                id,
                class: "ds-input",
                "data-variant": if bezeled { "bezeled" } else { "plain" },
                "data-kind": "multiline",
                "data-rows": rows.slug(),
                "aria-label": "{label}",
                "aria-multiline": "true",
                "aria-placeholder": aria_placeholder(&placeholder),
                "aria-invalid": invalid,
                "aria-description": description,
                "aria-disabled": availability.aria_disabled(),
                "aria-busy": availability.aria_busy(),
                autofocus: focus.on_mount().then_some("true"),
                value: "{value}",
                onmounted: move |event| {
                    caret.mounted(&event);
                    ime.mounted(&event);
                    focuser.mounted(focus, &event, told);
                },
                onfocus: move |_| handlers.onfocus.call(()),
                onblur: move |_| {
                    ime.left();
                    handlers.onchange.call(());
                    handlers.onblur.call(());
                },
                onmousedown: move |_| blink.restart(),
                oninput: move |event| {
                    blink.restart();
                    if availability == Availability::Enabled {
                        handlers.oninput.call(event.value());
                    }
                },
                onkeydown: move |event: KeyboardEvent| {
                    blink.restart();
                    // Enter is this field's newline: no outer default button takes it.
                    if event.key() == Key::Enter {
                        event.stop_propagation();
                    }
                    handlers.onkey.call(event);
                },
            }
            if let Some(text) = shown {
                span { class: "ds-input-placeholder", "aria-hidden": "true", "{text}" }
            }
        }
    }
}
