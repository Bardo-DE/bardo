//! Opening with an app the person picked, and the sheet that lists every app.

use crate::browser::Browser;
use crate::effect::{Effect, OpenTarget};
use crate::open::{Opens, opens};

type Stepped = (Browser, Vec<Effect>);

/// An app the person picked to open files with, by the id of its desktop entry. The model never
/// looks inside: the caller that launches apps knows what it means.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AppPick(String);

impl AppPick {
    /// The app whose desktop entry has this id (`org.gnome.TextEditor.desktop`).
    pub fn new(id: impl Into<String>) -> AppPick {
        AppPick(id.into())
    }

    /// The id as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Browser {
    /// Whether the sheet of every installed app is up.
    #[must_use]
    pub fn chooser_open(&self) -> bool {
        self.chooser
    }

    /// Open the selection with `app`: files and folders alike go to it, anything that opens
    /// nothing is left out. The sheet of apps, if up, goes away.
    pub(crate) fn open_with(self, app: AppPick) -> Stepped {
        let browser = Browser {
            chooser: false,
            ..self
        };
        let targets: Vec<OpenTarget> = browser
            .selected_rows()
            .into_iter()
            .filter(|(row, _)| opens(row) != Opens::Nothing)
            .map(|(_, item)| OpenTarget::new(item.folder, item.name))
            .collect();
        if targets.is_empty() {
            (browser, Vec::new())
        } else {
            (browser, vec![Effect::OpenFilesWith { targets, app }])
        }
    }

    /// Put up the sheet of every installed app, when something is selected in a folder.
    pub(crate) fn choose_app(self) -> Stepped {
        let chooser = !self.selected_targets().is_empty();
        (Browser { chooser, ..self }, Vec::new())
    }

    /// Put the sheet of apps away.
    pub(crate) fn close_chooser(self) -> Stepped {
        (
            Browser {
                chooser: false,
                ..self
            },
            Vec::new(),
        )
    }

    /// The selected items, by where they are, in shown order. In the Trash there are none.
    #[must_use]
    pub fn selected_targets(&self) -> Vec<OpenTarget> {
        self.selected_rows()
            .into_iter()
            .map(|(_, item)| OpenTarget::new(item.folder, item.name))
            .collect()
    }
}
