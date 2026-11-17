//! Three to five words: a segmented control.

use dioxus::prelude::*;
#[cfg_attr(
    test,
    expect(
        redundant_imports,
        reason = "the library build has no `toml` of its own; only the test build, which has it as a dev-dependency, finds this import redundant"
    )
)]
use settings_app_model::external::toml;
use settings_app_model::word_label;
use ui_components::components::controls::segmented::Tracking;
use ui_components::prelude::*;
use ui_components::style::tokens::control_size::ControlSize;
use settings_types::schema::WordLabels;

/// One segment per word, the stored word selected.
#[component]
pub fn SegmentedKey(
    label: String,
    words: Vec<String>,
    labels: WordLabels,
    value: toml::Value,
    onchange: EventHandler<toml::Value>,
) -> Element {
    let current = value.as_str().unwrap_or_default().to_owned();
    rsx! {
        SegmentedControl::<String> {
            label,
            choices: Choice::pairs(words.iter().map(|word| (word.clone(), word_label(&labels, word)))),
            tracking: Tracking::SelectOne(current),
            size: ControlSize::Small,
            onchange: move |word| onchange.call(toml::Value::String(word)),
        }
    }
}
