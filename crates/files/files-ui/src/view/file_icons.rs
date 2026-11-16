//! The window's file icons: the resolver is built when the window opens and again whenever the
//! person's choice of file icons (`appearance.file_icons`) changes.

use dioxus::prelude::*;
use ui_settings::schema::Provided;

use crate::icons::Icons;
use crate::launch::Launch;

/// The icons for the pack `provided` names, as a signal the views read. The standard set draws
/// until the pack has been found.
pub(super) fn use_file_icons(launch: &Launch, provided: &Provided) -> Signal<Icons> {
    let provided = provided.clone();
    let sources = launch.file_icons.clone();
    let places = launch.places.clone();
    let store = launch.appearance.store.clone();
    let mut icons =
        use_signal(|| Icons::standard(std::sync::Arc::clone(sources.db()), places.clone()));
    use_effect(use_reactive!(|provided| {
        let built = sources.build(&store, &provided);
        icons.set(Icons::new(
            std::sync::Arc::clone(sources.db()),
            built.icons,
            places.clone(),
        ));
    }));
    icons
}
