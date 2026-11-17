//! The mail-app states, as data: the name each renders under and how to make it.

use super::scoped::Scoped;
use dioxus::prelude::*;
use ui_components::components::controls::button_model::{Bezel, ImagePosition};
use ui_components::components::lists::row::row::Outline;
use ui_components::prelude::*;
use ui_components::root::pass_through::{DataAttr, DataName, ExtraClass};
use ui_core::vocab::RowState;
use ui_style::tokens::control_size::ControlSize;

/// One state and its name.
pub struct Case {
    pub name: &'static str,
    pub make: fn() -> Element,
}

/// `data-folder="<path>"`, as mailo's folder rows carry it.
fn folder(path: &str) -> Vec<DataAttr> {
    match DataName::parse("folder") {
        Ok(name) => vec![DataAttr::new(name, path)],
        Err(error) => panic!("{error}"),
    }
}

/// A consumer class, checked.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the result fills `Common::extra_class`, which is an `Option`"
)]
fn class(list: &str) -> Option<ExtraClass> {
    match ExtraClass::parse(list) {
        Ok(class) => Some(class),
        Err(error) => panic!("{error}"),
    }
}

/// The ⋯ for a folder row.
fn more(name: &str) -> Element {
    rsx! {
        Button { bezel: Bezel::Toolbar, image: ImagePosition::Only, icon: Icon::Ellipsis, label: "Actions for {name}", stop_propagation: true, onclick: |_| {} }
    }
}

/// Projects, `open`, in `drop` state, over one subfolder.
fn projects(open: bool, drop: DropState) -> Element {
    rsx! {
        Scoped { Row {
            state: RowState { drop, ..RowState::default() },
            title: "Projects",
            leading: RowLeading::Icon(Icon::Folder),
            outline: Outline::Branch(open),
            on_toggle: |_| {},
            accessory: Accessory::Slot(more("Projects")),
            onclick: |_| {},
            Row { title: "Bardo", leading: RowLeading::Icon(Icon::Folder), outline: Outline::Leaf }
        } }
    }
}

pub const CASES: &[Case] = &[
    Case {
        name: "controls/button/data-attr",
        make: || rsx! { Button { common: Common { data: folder("INBOX/Receipts"), ..Common::default() }, bezel: Bezel::Toolbar, label: "Receipts", onclick: |_| {} } },
    },
    Case {
        name: "controls/button/extra-class",
        make: || rsx! { Button { common: Common { extra_class: class("row-reveal  quiet-until-hover"), ..Common::default() }, size: ControlSize::Mini, label: "Reply", onclick: |_| {} } },
    },
    Case {
        name: "controls/button/data-attr-extra-class",
        make: || rsx! { Button { common: Common { data: folder("INBOX/Receipts"), extra_class: class("fold-more"), ..Common::default() }, bezel: Bezel::Toolbar, image: ImagePosition::Only, icon: Icon::Ellipsis, label: "Actions for Receipts", stop_propagation: true, onclick: |_| {} } },
    },
    Case {
        name: "lists/row/outline-open-idle",
        make: || projects(true, DropState::Idle),
    },
    Case {
        name: "lists/row/outline-open-accepts",
        make: || projects(true, DropState::Accepts),
    },
    Case {
        name: "lists/row/outline-open-target",
        make: || projects(true, DropState::Target),
    },
    Case {
        name: "lists/row/outline-open-source",
        make: || projects(true, DropState::Source),
    },
    Case {
        name: "lists/row/outline-closed-idle",
        make: || projects(false, DropState::Idle),
    },
    Case {
        name: "lists/row/outline-closed-accepts",
        make: || projects(false, DropState::Accepts),
    },
    Case {
        name: "lists/row/outline-closed-target",
        make: || projects(false, DropState::Target),
    },
    Case {
        name: "lists/row/outline-closed-source",
        make: || projects(false, DropState::Source),
    },
    Case {
        name: "lists/row/outline-leaf-current",
        make: || rsx! { Row { state: RowState { selection: true, ..RowState::default() }, title: "Receipts", outline: Outline::Leaf, accessory: Accessory::Slot(more("Receipts")) } },
    },
];
