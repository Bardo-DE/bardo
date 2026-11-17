//! Open With, Other: a Bardo `Sheet` listing every installed app. Choosing one opens the
//! selection with it.

use dioxus::prelude::*;
use files_model::{AppPick, Input};
use ui_components::base::vocab::RowState;
use ui_components::components::controls::button_model::Answers;
use ui_components::prelude::{
    Button, Column, ColumnAlign, Form, Icon, IconSize, List, ListItem, Row, RowLeading, Sheet,
};

use crate::menu::{AppLine, app_names};
use crate::state::Browse;

use super::icon::app_icon;

/// The sheet of apps, while it is up.
#[component]
pub(super) fn AppChooser(browse: Browse) -> Element {
    if browse.state.read().chooser_open() {
        rsx! {
            ChooserSheet { browse }
        }
    } else {
        rsx! {}
    }
}

/// One app's row.
#[component]
fn AppRow(line: AppLine, selection: bool, onpick: EventHandler<AppPick>) -> Element {
    let pick = line.pick.clone();
    // An app with no picture shows the UI kit's plain app glyph, so the names stay in one column.
    let leading = line
        .icon
        .as_deref()
        .and_then(|file| app_icon(file, IconSize::Large))
        .map_or(RowLeading::Icon(Icon::Window), RowLeading::Source);
    rsx! {
        Row {
            leading,
            title: line.title,
            state: RowState {
                selection,
                ..RowState::default()
            },
            onclick: move |_| onpick.call(pick.clone()),
        }
    }
}

/// The sheet itself: the list of apps and Cancel.
#[component]
fn ChooserSheet(browse: Browse) -> Element {
    let mut cursor = use_signal(|| None::<AppPick>);
    let lines = app_names(browse.all_apps());
    let cancel = EventHandler::new(move |()| browse.send(Input::CloseChooseApp));
    let pick = EventHandler::new(move |app: AppPick| browse.send(Input::OpenWith(app)));
    let select = EventHandler::new(move |app: AppPick| cursor.set(Some(app)));
    let here = cursor();
    let items: Vec<ListItem<AppPick>> = lines
        .into_iter()
        .map(|line| {
            let selection = here.as_ref() == Some(&line.pick);
            let (key, label) = (line.pick.clone(), line.title.clone());
            ListItem::row(
                key,
                label,
                rsx! { AppRow { line, selection, onpick: pick } },
            )
        })
        .collect();
    rsx! {
        Sheet {
            label: "Choose Application".to_owned(),
            onclose: cancel,
            Form {
                List::<AppPick> {
                    label: "Applications".to_owned(),
                    items,
                    cursor: here,
                    onselect: Some(select),
                    onpick: Some(pick),
                }
                Column { align: ColumnAlign::End,                     Button {
                        label: "Cancel",
                        answers: Answers::Escape,
                        onclick: move |_| cancel.call(()),
                    }
                }
            }
        }
    }
}
