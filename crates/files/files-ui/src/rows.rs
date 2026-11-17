//! The model's rows as Bardo's table wants them: columns, the sort, the cursor and the selection,
//! by row index. Pure.

use std::collections::HashMap;

use files_core::{FileName, Location, SortKey, SortOrder};
use files_model::{Browser, Input, Modifier};
use ui_components::base::selection::{Pick, PickHow, RowSelection};
use ui_components::components::lists::table::model::SortDirection;

use crate::words;

/// The table's columns, left to right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Column {
    /// A column the folder can be sorted by: a press on its header sorts by the key. The name is
    /// pinned, so it stays while the others scroll sideways.
    Sort(SortKey),
    /// Where a trashed item was before it was thrown away. Only in the Trash; its header does not
    /// sort.
    WhereFrom,
}

impl Column {
    /// Every column of a folder, in order.
    pub const ALL: [Column; 4] = [
        Column::Sort(SortKey::Name),
        Column::Sort(SortKey::Modified),
        Column::Sort(SortKey::Size),
        Column::Sort(SortKey::Kind),
    ];

    /// Every column of the Trash, in order.
    pub const TRASH: [Column; 5] = [
        Column::Sort(SortKey::Name),
        Column::WhereFrom,
        Column::Sort(SortKey::Modified),
        Column::Sort(SortKey::Size),
        Column::Sort(SortKey::Kind),
    ];

    /// The columns `location` is shown with.
    #[must_use]
    pub fn of_location(location: &Location) -> &'static [Column] {
        match location {
            Location::Trash => &Column::TRASH,
            Location::Folder(_) => &Column::ALL,
        }
    }

    /// The header's words.
    #[must_use]
    pub fn title(self) -> &'static str {
        words::column_title(self)
    }

    /// The header's words in `location`: the date in the Trash is when the item was thrown away.
    #[must_use]
    pub fn title_in(self, location: &Location) -> &'static str {
        words::column_title_in(self, location)
    }

    /// The key a press on this column's header sorts by; `None` for a column that does not sort.
    #[must_use]
    pub fn sort_key(self) -> Option<SortKey> {
        match self {
            Column::Sort(key) => Some(key),
            Column::WhereFrom => None,
        }
    }
}

/// The table's direction for a model's order.
#[must_use]
pub fn direction_of(order: SortOrder) -> SortDirection {
    match order {
        SortOrder::Ascending => SortDirection::Ascending,
        SortOrder::Descending => SortDirection::Descending,
    }
}

/// The model's order for a table's direction.
#[must_use]
pub fn order_of(direction: SortDirection) -> SortOrder {
    match direction {
        SortDirection::Ascending => SortOrder::Ascending,
        SortDirection::Descending => SortOrder::Descending,
    }
}

/// What a click's reach means to the model's selection.
#[must_use]
pub fn modifier_of(how: PickHow) -> Modifier {
    match how {
        PickHow::Replace => Modifier::None,
        PickHow::Toggle => Modifier::Toggle,
        PickHow::Extend => Modifier::Extend,
    }
}

/// The click a view's pick stands for: the row's name with how it was reached. `None` for a row
/// the browser no longer lists.
#[must_use]
pub fn click_of(browser: &Browser, pick: Pick) -> Option<Input> {
    let row = browser.rows().get(pick.row)?;
    Some(Input::Click(row.name.clone(), modifier_of(pick.how)))
}

/// The index of each row's name, to look many up at once.
fn indexes(browser: &Browser) -> HashMap<&FileName, usize> {
    browser
        .rows()
        .iter()
        .enumerate()
        .map(|(at, row)| (&row.name, at))
        .collect()
}

/// The row the keyboard is on.
#[must_use]
pub fn cursor_of(browser: &Browser) -> Option<usize> {
    let name = browser.cursor()?;
    browser.rows().iter().position(|row| row.name == *name)
}

/// The selected rows by index, with the anchor of a Shift-click and the cursor.
#[must_use]
pub fn selection_of(browser: &Browser) -> RowSelection {
    let index = indexes(browser);
    let at = |name: Option<&FileName>| name.and_then(|name| index.get(name).copied());
    let selection = browser.selection();
    let rows = selection
        .names()
        .filter_map(|name| index.get(name).copied());
    RowSelection::of_rows(at(selection.anchor()), at(selection.cursor()), rows)
}
