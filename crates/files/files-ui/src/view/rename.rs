//! The name field of a rename in place.

use dioxus::prelude::*;
use files_model::{Input, RenameError};
use ui_components::components::fields::text_field_model::Invalid;
use ui_components::motion::detail::stamp::EventStamp;
use ui_components::prelude::{FieldFocus, TextField, TextLine, Validity};

use crate::state::Browse;

/// The UI kit's text field in the name cell, in its plain bezel so the name is edited where it reads.
/// Return commits, Escape puts the name back, and leaving the field commits. A name the model
/// refuses stays here with its reason under it.
#[component]
pub(super) fn RenameCell(browse: Browse, draft: String, problem: Option<RenameError>) -> Element {
    let validity = match problem {
        Some(problem) => Validity::Invalid(Invalid {
            message: TextLine::from(problem.to_string()),
            stamp: EventStamp(0),
        }),
        None => Validity::Valid,
    };
    rsx! {
        TextField {
            label: "Name".to_owned(),
            value: draft,
            bezeled: false,
            focus: FieldFocus::OnMount,
            validity,
            oninput: move |text: String| browse.send(Input::EditDraft(text)),
            onsubmit: move |_: String| browse.send(Input::CommitRename),
            onblur: move |()| browse.send(Input::CommitRename),
            onkey: move |event: KeyboardEvent| {
                if event.key() == Key::Named(NamedKey::Escape) {
                    browse.send(Input::CancelRename);
                }
            },
        }
    }
}
