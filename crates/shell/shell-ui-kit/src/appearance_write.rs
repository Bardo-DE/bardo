//! The one writer of `bardo/appearance.toml`: a view asks, this function writes, and the
//! daemon's watch of the file then publishes the change to every surface.

use ui_components::prelude::Appearance;
use settings_types::AppName;
use ui_settings::AppearanceFile;
use settings_store_fs::{ConfigRoot, Store};

/// Write `next`'s theme, accent and motion into `bardo/appearance.toml`, keeping everything else
/// in the file. A file that cannot be written leaves the appearance as it was, which the picker
/// then shows.
pub fn write_appearance(next: Appearance) {
    let store = Store::new(ConfigRoot::Xdg, AppName::QUIRE);
    let file = with_appearance(store.load::<AppearanceFile>().value, next);
    if let Err(error) = store.save(&file) {
        eprintln!("bardo-shell: writing the appearance: {error}");
    }
}

/// `file` with `next`'s theme, accent and motion.
fn with_appearance(file: AppearanceFile, next: Appearance) -> AppearanceFile {
    let mut file = file;
    file.appearance.theme = next.theme;
    file.appearance.accent = next.accent;
    file.appearance.motion_level = next.motion;
    file
}

#[cfg(test)]
mod tests {
    use super::with_appearance;
    use ui_components::prelude::*;
    use ui_settings::AppearanceFile;

    #[test]
    fn a_pick_changes_only_the_three_fields() {
        let next = Appearance {
            theme: Theme::Dark,
            accent: Accent::ALL[1],
            motion: Motion::Reduced,
            ..Appearance::default()
        };
        let file = with_appearance(AppearanceFile::default(), next);
        assert_eq!(file.appearance.appearance(), next);
        assert_eq!(file.icons, AppearanceFile::default().icons);
    }
}
