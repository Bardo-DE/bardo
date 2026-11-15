//! A `true` or `false`: a switch.

use detent_model::external::toml;
use dioxus::prelude::*;
use ds::prelude::*;

/// On when the value is `true`.
#[component]
pub fn ToggleControl(
    label: String,
    value: toml::Value,
    onchange: EventHandler<toml::Value>,
) -> Element {
    let on = value.as_bool() == Some(true);
    rsx! {
        Toggle {
            label,
            value: if on { Check::On } else { Check::Off },
            onchange: move |next| onchange.call(toml::Value::Boolean(next == Check::On)),
        }
    }
}
