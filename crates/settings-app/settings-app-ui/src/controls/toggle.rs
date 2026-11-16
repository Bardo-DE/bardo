//! A `true` or `false`: a switch.

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
