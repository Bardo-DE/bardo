//! The window driven through the harness: pages, toggles, choices and search, on scratch
//! directories. Every write is read back from the scratch config home.

use crate::support::Sandbox;
use crate::support::drive::{click, page_of, search_for, segment, sidebar_page, title_of};
use settings_app_files::load;
use settings_app_model::PageSlug;
use settings_app_ui::Opening;
use ui_components::prelude::Word;
use ui_components::prelude::{Point, Px};
use ui_harness::Input;
use std::time::Duration;

mod controls;
mod pages;
mod search;

fn opening(page: &str) -> Opening {
    Opening {
        page: Some(PageSlug(page.to_owned())),
        query: String::new(),
    }
}

/// The `n`th picture choice of the theme row (1 Light, 2 Dark, 3 the one that follows the desktop).
fn theme_choice(n: usize) -> String {
    format!("[*|data-key=\"appearance.theme\"] .ds-radio-group-item:nth-of-type({n})")
}

fn harness_spec(sandbox: &Sandbox) -> settings_app_model::FileSpec {
    sandbox
        .env(Opening::default(), ui_components::prelude::Scheme::Light)
        .catalogue
        .files
        .iter()
        .find(|f| {
            f.file.relative().ends_with("settings.toml") && f.file.relative().starts_with("sill")
        })
        .cloned()
        .expect("sill's file")
}

fn detent_model_key(path: &str) -> ui_settings::schema::KeyPath {
    ui_settings::schema::KeyPath(path.to_owned())
}
