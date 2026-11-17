//! A `data-*` mark on a Bardo component, which the window's tests address a row by.

use ui_components::root::common::Common;
use ui_components::root::pass_through::{DataAttr, DataName};

/// The `common` of a component that carries `data-<name>="<value>"`; none when `name` is not a
/// valid attribute name.
pub(crate) fn marked(name: &'static str, value: &str) -> Common {
    Common {
        data: DataName::parse(name)
            .map(|name| vec![DataAttr::new(name, value.to_owned())])
            .unwrap_or_default(),
        ..Common::default()
    }
}
