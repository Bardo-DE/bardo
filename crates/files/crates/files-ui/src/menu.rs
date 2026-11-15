//! What the context menu offers: pure, so a table can say it.

use std::path::PathBuf;

use files_core::Location;
use files_model::{AppPick, Browser, Input};

use crate::edge::AppChoiceView;
use crate::words;

/// quire's availability for a yes/no.
pub fn enabled(can: bool) -> ds::prelude::Availability {
    if can {
        ds::prelude::Availability::Enabled
    } else {
        ds::prelude::Availability::Disabled
    }
}

/// One command of the context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    /// Open the selection.
    Open,
    /// Open the selected folder in a new tab.
    OpenInNewTab,
    /// Open the selection with an app picked from a submenu. It is a parent, never picked itself.
    OpenWith,
    /// Open Quick Look on the selection.
    QuickLook,
    /// Show the properties of the selection.
    GetInfo,
    /// Show the properties of the folder shown.
    GetInfoFolder,
    /// Put the full paths of the selection on the clipboard.
    CopyAsPathname,
    /// Move the selection to the Trash.
    MoveToTrash,
    /// Rename the selected item.
    Rename,
    /// Copy the selection.
    Copy,
    /// Paste what was copied into this folder.
    Paste,
    /// Make a new folder.
    NewFolder,
    /// Put the selection back where it came from (in the Trash).
    PutBack,
    /// Erase the selection for good (in the Trash).
    DeleteImmediately,
    /// Erase everything in the Trash for good.
    EmptyTrash,
    /// Download the selection now (in a cloud drive).
    DownloadNow,
    /// Remove the downloaded copy of the selection, keeping it listed (in a cloud drive).
    RemoveDownload,
    /// Keep the selection downloaded; checked while all of it is (in a cloud drive).
    KeepDownloaded,
    /// Save the online documents of the selection as files next to them (in a cloud drive).
    SaveCopy,
    /// Show the preview pane beside the folder.
    ShowPreview,
    /// Put the preview pane away.
    HidePreview,
}

impl Command {
    /// The menu item's words.
    pub fn title(self) -> &'static str {
        words::command_title(self)
    }

    /// What picking it asks of the browser; `None` for a parent of a submenu, which is not picked.
    pub fn input(self) -> Option<Input> {
        Some(match self {
            Command::OpenWith => return None,
            Command::Open => Input::Open,
            Command::OpenInNewTab => Input::OpenInNewTab,
            Command::QuickLook => Input::QuickLook,
            Command::GetInfo => Input::GetInfo,
            Command::GetInfoFolder => Input::GetInfoFolder,
            Command::CopyAsPathname => Input::CopyAsPathname,
            Command::MoveToTrash => Input::MoveToTrash,
            Command::Rename => Input::BeginRename,
            Command::Copy => Input::Copy,
            Command::Paste => Input::Paste,
            Command::NewFolder => Input::NewFolder,
            Command::PutBack => Input::PutBack,
            Command::DeleteImmediately => Input::DeleteImmediately,
            Command::EmptyTrash => Input::EmptyTrash,
            Command::DownloadNow => Input::DownloadNow,
            Command::RemoveDownload => Input::RemoveDownload,
            Command::KeepDownloaded => Input::KeepDownloaded,
            Command::SaveCopy => Input::SaveCopy,
            Command::ShowPreview | Command::HidePreview => Input::TogglePreview,
        })
    }
}

/// What a menu line yields when picked: a command, an app of the Open With submenu, or "Other…".
#[derive(Debug, Clone, PartialEq)]
pub enum Pick {
    /// A command of the menu.
    Command(Command),
    /// An app to open the selection with.
    App(AppPick),
    /// Choose from every installed app.
    OtherApp,
}

impl Pick {
    /// What picking it asks of the browser; `None` for a parent of a submenu.
    pub fn input(&self) -> Option<Input> {
        match self {
            Pick::Command(command) => command.input(),
            Pick::App(app) => Some(Input::OpenWith(app.clone())),
            Pick::OtherApp => Some(Input::ChooseApp),
        }
    }
}

pub use crate::words::OTHER_APP;

/// One app line of the Open With submenu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppLine {
    /// What it yields.
    pub pick: AppPick,
    /// Its words: the name, and "(default)" after the default's.
    pub title: String,
    /// The image file of its icon, when it has one.
    pub icon: Option<PathBuf>,
}

/// The Open With lines for `apps`: the default first with "(default)" after its name, the others
/// sorted by name (ignoring case).
pub fn app_lines(apps: Vec<AppChoiceView>) -> Vec<AppLine> {
    let (mut defaults, mut others): (Vec<_>, Vec<_>) =
        apps.into_iter().partition(|app| app.is_default);
    others.sort_by_cached_key(|app| (app.name.to_lowercase(), app.name.clone()));
    defaults.truncate(1);
    defaults
        .into_iter()
        .map(|app| AppLine {
            title: words::default_app_title(&app.name),
            pick: app.pick,
            icon: app.icon,
        })
        .chain(others.into_iter().map(|app| AppLine {
            title: app.name,
            pick: app.pick,
            icon: app.icon,
        }))
        .collect()
}

/// Every app for the sheet behind "Other…", sorted by name (ignoring case), with no default mark.
pub fn app_names(apps: Vec<AppChoiceView>) -> Vec<AppLine> {
    let mut lines: Vec<AppLine> = apps
        .into_iter()
        .map(|app| AppLine {
            pick: app.pick,
            title: app.name,
            icon: app.icon,
        })
        .collect();
    lines.sort_by_cached_key(|line| (line.title.to_lowercase(), line.title.clone()));
    lines
}

/// Where the press was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Over {
    /// On a row (which the press has selected).
    Row,
    /// On the folder's background.
    Background,
}

/// The menu for a press `over` something in `browser`: groups of commands, each with whether it
/// can be picked now.
pub fn groups_for(browser: &Browser, over: Over) -> Vec<Vec<(Command, bool)>> {
    let selected = browser.selection().len();
    let some = selected > 0;
    let one = selected == 1;
    let rows = !browser.rows().is_empty();
    match (browser.location(), over) {
        (Location::Trash, Over::Row) => vec![vec![
            (Command::PutBack, some),
            (Command::DeleteImmediately, some),
        ]],
        (Location::Trash, Over::Background) => vec![vec![(Command::EmptyTrash, rows)]],
        (_, Over::Row) => {
            let mut groups = vec![open_group(browser, some)];
            groups.extend(cloud_group(browser));
            groups.extend([
                vec![(Command::MoveToTrash, some)],
                vec![
                    (Command::GetInfo, some),
                    (Command::Rename, one),
                    (Command::QuickLook, some),
                ],
                vec![(Command::Copy, some), (Command::CopyAsPathname, some)],
                vec![(Command::NewFolder, true)],
            ]);
            groups
        }
        (_, Over::Background) => vec![
            vec![(Command::NewFolder, true)],
            vec![(Command::GetInfoFolder, true)],
            vec![(Command::Paste, browser.can_paste())],
            vec![(preview_command(browser), true)],
        ],
    }
}

/// Download Now, Remove Download and Keep Downloaded, for a selection in a cloud drive.
fn cloud_group(browser: &Browser) -> Option<Vec<(Command, bool)>> {
    let menu = browser.cloud_menu()?;
    let mut group = vec![
        (Command::DownloadNow, menu.download),
        (Command::RemoveDownload, menu.remove),
        (Command::KeepDownloaded, menu.keep),
    ];
    if menu.copy {
        group.push((Command::SaveCopy, menu.copy));
    }
    Some(group)
}

/// Show Preview while the pane is away, Hide Preview while it is up.
fn preview_command(browser: &Browser) -> Command {
    if browser.pane_shown() {
        Command::HidePreview
    } else {
        Command::ShowPreview
    }
}

/// Open, Open in New Tab (only on a lone folder, right after Open) and Open With.
fn open_group(browser: &Browser, some: bool) -> Vec<(Command, bool)> {
    let mut group = vec![(Command::Open, some)];
    if browser.can_open_in_new_tab() {
        group.push((Command::OpenInNewTab, true));
    }
    group.push((Command::OpenWith, some));
    group
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use files_core::{
        ByteSize, Entry, EntryKind, FileName, FolderPath, ItemRef, TrashId, TrashedItem,
    };
    use files_model::{Modifier, Now};

    use super::*;

    fn name(text: &str) -> FileName {
        FileName::new(text).unwrap()
    }

    fn step(browser: Browser, input: Input) -> Browser {
        browser.step(input, Now(Duration::ZERO)).0
    }

    fn folder_browser() -> Browser {
        let here = Location::Folder(FolderPath::new("/home/p").unwrap());
        let (browser, _) = Browser::new(here.clone());
        let rows = vec![
            Entry::new(name("a.txt"), EntryKind::File(ByteSize(1))),
            Entry::new(name("b.txt"), EntryKind::File(ByteSize(1))),
        ];
        step(browser, Input::Listed(here, Ok(rows)))
    }

    fn trash_browser() -> Browser {
        let (browser, _) = Browser::new(Location::Trash);
        let kind = EntryKind::File(ByteSize(1));
        let original = ItemRef::new(FolderPath::new("/home/p").unwrap(), name("a.txt"));
        let item = TrashedItem::new(
            TrashId::new("/t/info/a.txt.trashinfo"),
            original,
            SystemTime::UNIX_EPOCH,
            kind.clone(),
        );
        let rows = vec![Entry::new(name("a.txt"), kind).in_trash(item)];
        step(browser, Input::Listed(Location::Trash, Ok(rows)))
    }

    fn click(browser: Browser, text: &str) -> Browser {
        step(browser, Input::Click(name(text), Modifier::None))
    }

    type Group = Vec<(Command, bool)>;

    #[test]
    fn a_row_in_a_folder_offers_what_the_selection_allows_in_finders_order() {
        let one = click(folder_browser(), "a.txt");
        let want: Vec<Group> = vec![
            vec![(Command::Open, true), (Command::OpenWith, true)],
            vec![(Command::MoveToTrash, true)],
            vec![
                (Command::GetInfo, true),
                (Command::Rename, true),
                (Command::QuickLook, true),
            ],
            vec![(Command::Copy, true), (Command::CopyAsPathname, true)],
            vec![(Command::NewFolder, true)],
        ];
        assert_eq!(groups_for(&one, Over::Row), want);

        // Two selected: nothing to rename.
        let two = step(one, Input::Click(name("b.txt"), Modifier::Toggle));
        let rename = groups_for(&two, Over::Row)[2][1];
        assert_eq!(rename, (Command::Rename, false));
    }

    #[test]
    fn the_background_of_a_folder_makes_folders_shows_its_info_pastes_and_toggles_the_preview() {
        let copied = step(click(folder_browser(), "a.txt"), Input::Copy);
        let want: Vec<Group> = vec![
            vec![(Command::NewFolder, true)],
            vec![(Command::GetInfoFolder, true)],
            vec![(Command::Paste, true)],
            vec![(Command::ShowPreview, true)],
        ];
        assert_eq!(groups_for(&copied, Over::Background), want);
        // With the pane up the same line puts it away.
        let up = step(copied, Input::TogglePreview);
        assert_eq!(
            groups_for(&up, Over::Background)[3],
            vec![(Command::HidePreview, true)]
        );
        assert_eq!(
            groups_for(&folder_browser(), Over::Background)[2],
            vec![(Command::Paste, false)]
        );
    }

    #[test]
    fn the_trash_offers_put_back_and_erasing() {
        let selected = click(trash_browser(), "a.txt");
        let row: Vec<Group> = vec![vec![
            (Command::PutBack, true),
            (Command::DeleteImmediately, true),
        ]];
        assert_eq!(groups_for(&selected, Over::Row), row);
        let background: Vec<Group> = vec![vec![(Command::EmptyTrash, true)]];
        assert_eq!(groups_for(&selected, Over::Background), background);
    }

    #[test]
    fn each_command_asks_the_browser_for_its_input_and_says_its_words() {
        let table = [
            (Command::Open, Input::Open, "Open"),
            (
                Command::OpenInNewTab,
                Input::OpenInNewTab,
                "Open in New Tab",
            ),
            (Command::QuickLook, Input::QuickLook, "Quick Look"),
            (Command::GetInfo, Input::GetInfo, "Get Info"),
            (Command::GetInfoFolder, Input::GetInfoFolder, "Get Info"),
            (
                Command::CopyAsPathname,
                Input::CopyAsPathname,
                "Copy as Pathname",
            ),
            (Command::MoveToTrash, Input::MoveToTrash, "Move to Trash"),
            (Command::Rename, Input::BeginRename, "Rename"),
            (Command::Copy, Input::Copy, "Copy"),
            (Command::Paste, Input::Paste, "Paste"),
            (Command::NewFolder, Input::NewFolder, "New Folder"),
            (Command::PutBack, Input::PutBack, "Put Back"),
            (
                Command::DeleteImmediately,
                Input::DeleteImmediately,
                "Delete Immediately\u{2026}",
            ),
            (
                Command::EmptyTrash,
                Input::EmptyTrash,
                "Empty Trash\u{2026}",
            ),
            (Command::ShowPreview, Input::TogglePreview, "Show Preview"),
            (Command::HidePreview, Input::TogglePreview, "Hide Preview"),
        ];
        for (command, input, title) in table {
            assert_eq!(command.input(), Some(input));
            assert_eq!(command.title(), title);
        }
        // Open With is a parent: it opens a submenu and asks nothing itself.
        assert_eq!(Command::OpenWith.input(), None);
        assert_eq!(Command::OpenWith.title(), "Open With");
    }

    fn app(id: &str, name: &str, is_default: bool) -> AppChoiceView {
        AppChoiceView {
            pick: AppPick::new(id),
            name: name.to_owned(),
            is_default,
            icon: None,
        }
    }

    fn line(id: &str, title: &str) -> AppLine {
        AppLine {
            pick: AppPick::new(id),
            title: title.to_owned(),
            icon: None,
        }
    }

    #[test]
    fn open_with_lists_the_default_first_and_marks_it_then_the_rest_by_name() {
        let apps = vec![
            app("zed", "Zed", false),
            app("viewer", "Viewer", true),
            app("editor", "editor", false),
            app("calc", "Calculator", false),
        ];
        assert_eq!(
            app_lines(apps),
            vec![
                line("viewer", "Viewer (default)"),
                line("calc", "Calculator"),
                line("editor", "editor"),
                line("zed", "Zed"),
            ]
        );
        // With no default, the apps are just sorted.
        let plain = vec![app("b", "Bee", false), app("a", "Ant", false)];
        assert_eq!(app_lines(plain), vec![line("a", "Ant"), line("b", "Bee")]);
        assert!(app_lines(Vec::new()).is_empty());
    }

    #[test]
    fn an_apps_icon_goes_with_its_line_in_the_menu_and_the_sheet() {
        let with_icon = |mut view: AppChoiceView| {
            view.icon = Some(PathBuf::from("/icons/v.png"));
            view
        };
        let apps = || vec![with_icon(app("v", "Viewer", true)), app("z", "Zed", false)];
        let icons = |lines: Vec<AppLine>| -> Vec<(String, Option<PathBuf>)> {
            lines.into_iter().map(|l| (l.title, l.icon)).collect()
        };
        let want = vec![
            (
                "Viewer (default)".to_owned(),
                Some(PathBuf::from("/icons/v.png")),
            ),
            ("Zed".to_owned(), None),
        ];
        assert_eq!(icons(app_lines(apps())), want);
        let sheet: Vec<_> = want
            .iter()
            .map(|(title, icon)| (title.replace(" (default)", ""), icon.clone()))
            .collect();
        assert_eq!(icons(app_names(apps())), sheet);
    }

    #[test]
    fn the_other_sheet_lists_every_app_by_name_with_no_default_mark() {
        let apps = vec![app("z", "Zed", false), app("v", "Viewer", true)];
        assert_eq!(app_names(apps), vec![line("v", "Viewer"), line("z", "Zed")]);
    }

    #[test]
    fn a_menu_pick_asks_the_browser_for_its_input() {
        let table = [
            (Pick::Command(Command::Copy), Some(Input::Copy)),
            (Pick::Command(Command::OpenWith), None),
            (
                Pick::App(AppPick::new("viewer")),
                Some(Input::OpenWith(AppPick::new("viewer"))),
            ),
            (Pick::OtherApp, Some(Input::ChooseApp)),
        ];
        for (pick, input) in table {
            assert_eq!(pick.input(), input);
        }
        assert_eq!(OTHER_APP, "Other\u{2026}");
    }

    fn with_folder() -> Browser {
        let here = Location::Folder(FolderPath::new("/home/p").unwrap());
        let (browser, _) = Browser::new(here.clone());
        let rows = vec![
            Entry::new(name("Projects"), EntryKind::Folder),
            Entry::new(name("a.txt"), EntryKind::File(ByteSize(1))),
            Entry::new(name("b.txt"), EntryKind::File(ByteSize(1))),
        ];
        step(browser, Input::Listed(here, Ok(rows)))
    }

    #[test]
    fn open_in_new_tab_follows_open_on_a_lone_folder_and_is_left_out_otherwise() {
        let folder = click(with_folder(), "Projects");
        let first = groups_for(&folder, Over::Row)[0].clone();
        assert_eq!(
            first,
            vec![
                (Command::Open, true),
                (Command::OpenInNewTab, true),
                (Command::OpenWith, true),
            ]
        );
        let file = click(with_folder(), "a.txt");
        let first = groups_for(&file, Over::Row)[0].clone();
        assert_eq!(
            first,
            vec![(Command::Open, true), (Command::OpenWith, true)]
        );
        // Two folders-or-more selected: not a lone folder.
        let two = step(folder, Input::Click(name("a.txt"), Modifier::Toggle));
        assert_eq!(groups_for(&two, Over::Row)[0].len(), 2);
    }
}
