//! A colour: the accent swatch row.

use dioxus::prelude::*;
#[cfg_attr(
    test,
    expect(
        redundant_imports,
        reason = "the library build has no `toml` of its own; only the test build, which has it as a dev-dependency, finds this import redundant"
    )
)]
use settings_app_model::external::toml;
use ui_components::components::controls::radio_group::Arrangement;
use ui_components::prelude::*;
use ui_components::style::css::accents_css::swatch_var;

/// Whether every word is one of the accent colours: an enum of them is drawn as swatches.
pub fn swatch_words(words: &[String]) -> bool {
    !words.is_empty() && words.iter().all(|word| Accent::parse(word).is_some())
}

/// The eight accents as colour circles; the stored word is the accent's name. A stored word that
/// is not an accent selects none.
#[component]
pub fn SwatchControl(
    label: String,
    value: toml::Value,
    onchange: EventHandler<toml::Value>,
) -> Element {
    let current = value.as_str().and_then(Accent::parse);
    let choices: Vec<Choice<Option<Accent>>> = Accent::ALL
        .iter()
        .map(|accent| Choice::new(Some(*accent), accent.label()).with_swatch(swatch_var(*accent)))
        .collect();
    rsx! {
        RadioGroup::<Option<Accent>> {
            label,
            choices,
            value: current,
            arrangement: Arrangement::Swatches,
            onchange: move |picked: Option<Accent>| {
                if let Some(accent) = picked {
                    onchange.call(toml::Value::String(accent.slug().to_owned()));
                }
            },
        }
    }
}
