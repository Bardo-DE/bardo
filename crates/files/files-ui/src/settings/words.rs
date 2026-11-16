//! The words the settings are stored as: a closed `Word` enum, so the schema names every value.

use ui_components::prelude::Word;
use serde::{Deserialize, Serialize};

/// `general.new_windows_show`: the folder a new window opens on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum NewWindowsShow {
    /// The home folder.
    #[default]
    Home,
    /// The Desktop folder.
    Desktop,
    /// The Documents folder.
    Documents,
    /// The Downloads folder.
    Downloads,
    /// The folder `general.new_windows_folder` names.
    #[word(label = "A folder I choose")]
    Folder,
}
