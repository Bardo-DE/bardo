//! The locator over a reader of plain values: no disk.

use super::tests::base;
use super::*;
use settings_store::Loaded;
use settings_store_fs::{Locator, ReadFailure, ReadText};
use std::path::{Path, PathBuf};

/// The files a test shows the locator: a path with its text, or `None` for a file that is there
/// and cannot be read.
struct Files(Vec<(PathBuf, Option<String>)>);

impl ReadText for Files {
    fn read_text(&self, path: &Path) -> Result<String, ReadFailure> {
        self.0
            .iter()
            .find(|(at, _)| at == path)
            .map_or(Err(ReadFailure::Missing), |(_, shown)| match shown {
                Some(text) => Ok(text.clone()),
                None => Err(ReadFailure::Unreadable {
                    reason: "is a directory".to_owned(),
                }),
            })
    }
}

fn locator_for(home: &str, dirs: &str) -> Locator {
    locator(&|k: &str| match k {
        "XDG_CONFIG_HOME" => Some(home.to_owned()),
        "XDG_CONFIG_DIRS" => Some(dirs.to_owned()),
        _ => None,
    })
}

fn at(dir: &str, shown: Option<String>) -> (PathBuf, Option<String>) {
    (Path::new(dir).join("bardo").join(SETTINGS_FILE), shown)
}

#[test]
fn the_locator_reads_the_first_file_of_the_configuration_directories() {
    let files = Files(vec![
        at(
            "/home",
            Some("[computer_use.budget]\nspend_microusd = 900\n".to_owned()),
        ),
        at(
            "/etc",
            Some(
                "[computer_use.budget]\nspend_microusd = 1900\n[computer use]\nenabled = true\n"
                    .to_owned(),
            ),
        ),
    ]);
    let locator = locator_for("/home", "/etc");
    assert_eq!(
        locator.watch_dir(),
        Some(PathBuf::from("/home/bardo/computer-use"))
    );
    let value = locator.read_with(&files, base(), read).value;
    assert_eq!(
        value.config.budget.spend,
        model_provider::Money::usd(rust_decimal::Decimal::new(900, 6))
    );
    // The first file wins whole: the later directory's other keys are not merged in.
    assert!(!value.config.enabled);
    let none = super::locator(&|_: &str| None::<String>);
    assert_eq!(none.read_with(&files, base(), read), Loaded::clean(base()));
    // A settings file that is there and cannot be read is reported, and the next directory
    // still answers.
    let files = Files(vec![
        at("/blocked", None),
        at(
            "/etc",
            Some("[computer_use.budget]\nspend_microusd = 1900\n".to_owned()),
        ),
    ]);
    let loaded = locator_for("/blocked", "/etc").read_with(&files, base(), read);
    assert_eq!(
        loaded.value.config.budget.spend,
        model_provider::Money::usd(rust_decimal::Decimal::new(1900, 6))
    );
    assert_eq!(loaded.invalid.len(), 1);
    assert!(loaded.invalid[0].reason.starts_with("could not be read"));
    assert!(loaded.invalid[0].path.ends_with(SETTINGS_FILE));
}
