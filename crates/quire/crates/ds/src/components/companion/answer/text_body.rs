//! A text answer's lines.

use super::model::TextAnswer;
use crate::components::content::text_runs::{TextLine, text};
use dioxus::prelude::*;

/// The lines as the shell sets them, one paragraph each, runs in their own tones. While text is
/// still arriving the body says it is busy, so a screen reader waits for the whole.
#[component]
pub fn TextBody(answer: TextAnswer) -> Element {
    rsx! {
        div {
            class: "ds-card-text",
            "data-streaming": if answer.streaming { "arriving" } else { "complete" },
            "aria-live": "polite",
            "aria-busy": answer.streaming.then_some("true"),
            for (index , line) in answer.body.iter().enumerate() {
                Line { key: "{index}", line: line.clone() }
            }
        }
    }
}

#[component]
fn Line(line: TextLine) -> Element {
    rsx! {
        p { class: "ds-card-line", {text(&line)} }
    }
}
