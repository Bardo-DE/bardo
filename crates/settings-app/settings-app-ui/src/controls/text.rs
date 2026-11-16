//! Free text: a text field that stores when the caret leaves or Enter is pressed.

use dioxus::prelude::*;
#[cfg_attr(
    test,
    expect(
        redundant_imports,
        reason = "the library build has no `toml` of its own; only the test build, which has it as a dev-dependency, finds this import redundant"
    )
)]
use settings_app_model::external::toml;
use ui_components::prelude::*;
use ui_components::style::tokens::control_size::ControlSize;

/// A text field holding a draft while it is edited.
#[component]
pub fn TextControl(
    label: String,
    value: toml::Value,
    onchange: EventHandler<toml::Value>,
) -> Element {
    let stored = value.as_str().unwrap_or_default().to_owned();
    let mut draft = use_signal(|| stored.clone());
    rsx! {
        TextField {
            label,
            value: draft(),
            size: ControlSize::Small,
            oninput: move |next| draft.set(next),
            onchange: move |next: String| onchange.call(toml::Value::String(next)),
        }
    }
}
