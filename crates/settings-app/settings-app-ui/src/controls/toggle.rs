//! A `true` or `false`: a switch.

use dioxus::prelude::*;
use settings_app_model::external::toml;
use ui_components::prelude::*;

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
