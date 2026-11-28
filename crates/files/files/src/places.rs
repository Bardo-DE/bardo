//! The person's user folders, found once from the XDG settings (`user-dirs.dirs`).

use std::path::{Path, PathBuf};

use base_xdg::{UserDirs, Xdg};
use files_core::FolderPath;
use files_ui::settings::NewWindow;
use files_ui::{PlaceKind, Places};

/// The person's own folders, read from `user-dirs.dirs`; none are named when the file is missing or
/// there is no home.
fn user_dirs(xdg: &Xdg<impl Fn(&str) -> Option<String>>) -> UserDirs {
    let (Some(home), Ok(file)) = (xdg.home(), xdg.user_dirs_file()) else {
        return UserDirs::default();
    };
    std::fs::read_to_string(file)
        .map_or_else(|_| UserDirs::default(), |text| UserDirs::parse(&text, home))
}

/// The folder a new window opens on: the one chosen, or the home folder when that is not a folder
/// on this computer.
pub fn new_window(choice: NewWindow) -> Option<PathBuf> {
    let xdg = Xdg::process();
    let home = xdg.home().map(Path::to_path_buf);
    let user = user_dirs(&xdg);
    let named = |name: &str| user.get(name).map(Path::to_path_buf);
    let chosen = match choice {
        NewWindow::Home => home.clone(),
        NewWindow::Desktop => named("XDG_DESKTOP_DIR"),
        NewWindow::Documents => named("XDG_DOCUMENTS_DIR"),
        NewWindow::Downloads => named("XDG_DOWNLOAD_DIR"),
        NewWindow::Folder(path) => Some(expanded(&path, home.as_deref())),
    };
    chosen.filter(|folder| folder.is_dir()).or(home)
}

/// `path` with a leading `~` standing for `home`.
fn expanded(path: &Path, home: Option<&Path>) -> PathBuf {
    match (path.strip_prefix("~"), home) {
        (Ok(rest), Some(home)) => home.join(rest),
        _ => path.to_path_buf(),
    }
}

/// Home and the user folders that exist, as the sidebar's favorites.
pub fn found() -> Places {
    let xdg = Xdg::process();
    let user = user_dirs(&xdg);
    let named = |name: &str| user.get(name).map(Path::to_path_buf);
    let folders = [
        (PlaceKind::Home, xdg.home().map(Path::to_path_buf)),
        (PlaceKind::Desktop, named("XDG_DESKTOP_DIR")),
        (PlaceKind::Documents, named("XDG_DOCUMENTS_DIR")),
        (PlaceKind::Downloads, named("XDG_DOWNLOAD_DIR")),
        (PlaceKind::Pictures, named("XDG_PICTURES_DIR")),
        (PlaceKind::Music, named("XDG_MUSIC_DIR")),
        (PlaceKind::Videos, named("XDG_VIDEOS_DIR")),
    ];
    folders
        .into_iter()
        .filter_map(|(kind, path)| Some((kind, path.filter(|p| p.is_dir())?)))
        .filter_map(|(kind, path)| Some((kind, FolderPath::new(path).ok()?)))
        .fold(Places::default(), |places, (kind, folder)| {
            places.with(kind, folder)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_leading_tilde_is_the_home_folder() {
        let home = Some(Path::new("/home/p"));
        let cases = [
            ("a tilde", "~/Work", home, "/home/p/Work"),
            ("only the tilde", "~", home, "/home/p"),
            ("no home", "~/Work", None, "~/Work"),
            ("a full path", "/srv/files", home, "/srv/files"),
            ("a tilde inside a name", "~work/x", home, "~work/x"),
        ];
        for (name, path, home, want) in cases {
            assert_eq!(expanded(Path::new(path), home), Path::new(want), "{name}");
        }
    }
}
