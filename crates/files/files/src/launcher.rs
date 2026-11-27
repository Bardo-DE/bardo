//! Building the launcher from the person's installed apps and file associations.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use base_xdg::Xdg;
use files_desktop::{
    Activate, ActivationToken, Catalog, DesktopLauncher, StartError, installed_check, load_apps,
};
use ui_helpers::mime::{Associations, MimeDb, XdgDirs};

use crate::icons::ThemeIcons;

/// The XDG folders, read once from the environment (`$XDG_DATA_HOME` and friends, with the
/// specification's defaults).
pub fn xdg_dirs() -> XdgDirs {
    let xdg = Xdg::process();
    let home = xdg.home().map_or_default(Path::to_path_buf);
    XdgDirs {
        data_home: xdg
            .data_home()
            .unwrap_or_else(|_| home.join(".local/share")),
        data_dirs: xdg.data_dirs(),
        config_home: xdg.config_home().unwrap_or_else(|_| home.join(".config")),
        config_dirs: xdg.config_dirs(),
        current_desktops: std::env::var("XDG_CURRENT_DESKTOP")
            .map_or_default(|value| {
                value
                    .split(':')
                    .filter(|part| !part.is_empty())
                    .map(str::to_owned)
                    .collect()
            }),
    }
}

/// The file type database of the installed system. With none, every file is of an unknown type,
/// so the window says there is no app for it and draws the standard icon.
pub fn mime_db(dirs: &XdgDirs) -> MimeDb {
    MimeDb::load(dirs).map_or_else(|_| MimeDb::from_layers(&[]), |loaded| loaded.value)
}

/// The launcher over what is installed, with the types of `db`.
pub fn launcher(dirs: &XdgDirs, db: &MimeDb, activate: Arc<dyn Activate>) -> DesktopLauncher {
    let db = db.clone();
    let applications: Vec<PathBuf> = std::iter::once(&dirs.data_home)
        .chain(dirs.data_dirs.iter())
        .map(|dir| dir.join("applications"))
        .collect();
    let apps = load_apps(&applications);
    let associations = Associations::load(dirs, installed_check(&apps));
    let catalog = Catalog {
        db,
        associations,
        apps,
    };
    DesktopLauncher::builder(catalog, activate)
        .icons(Arc::new(ThemeIcons::from_settings(&dirs.config_home)))
        .build()
}

/// Starting apps over the bus when there is no bus to ask: every such app fails in words.
#[derive(Debug, Clone, Copy)]
pub struct NoBus;

impl Activate for NoBus {
    fn open(
        &self,
        _bus_name: &str,
        _uris: &[String],
        _token: Option<&ActivationToken>,
    ) -> Result<(), StartError> {
        Err(StartError::new("The desktop isn\u{2019}t answering."))
    }
}
