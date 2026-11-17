//! The shell's own sheets, linted as a consumer's CSS under the strictest profile (the coherence
//! rules, the UI kit's ARCHITECTURE.md "Repo rules"): the UI kit's sheets are the UI kit's to lint
//! (`ui-shell/tests/self_lint.rs`); these are the ones that moved here, so every rule a consumer
//! obeys holds for them, one sheet at a time. One rule is set aside whole: [`Rule::DsInternals`],
//! because these sheets style the `ds-*` classes of the components this crate draws (the markup
//! the shell has always rendered, which the specimens exercise). The details grammar's two rules are
//! judged per sheet in `details_lint.rs`.

use std::collections::HashSet;
use ui_lint::{LintConfig, Offence, Profile, Rule, stylesheet};

/// Custom properties a component writes inline per element, which no stylesheet block declares: a
/// notification group's layer count and a layer's place in it (`NotificationCard`), a swiped
/// card's offset (`NotificationCard { swipe }`), and an animated emoji's disc
/// (`AnimatedEmoji { disc }`).
const INLINE_VARS: &[&str] = &["--layers", "--i", "--swipe-dx", "--em-disc"];

/// The sheets this crate owns: every component sheet of the shell's stylesheet that Bardo's
/// (`ds` and `ui-shell`) do not list.
fn own_sheets() -> Vec<(&'static str, &'static str)> {
    let bardo: HashSet<&str> = ui_shell::component_sheets()
        .iter()
        .map(|(name, _)| *name)
        .collect();
    shell_kit::component_sheets()
        .into_iter()
        .filter(|(name, _)| !bardo.contains(name))
        .collect()
}

fn config() -> LintConfig {
    LintConfig {
        profile: Profile::Strict,
        own_vars: INLINE_VARS.iter().map(|name| (*name).to_owned()).collect(),
        ..LintConfig::new(&shell_kit::kits())
    }
}

/// Whether `offence` is on a `ds-*` class this crate's own components wear.
fn own_class(offence: &Offence) -> bool {
    offence.rule == Rule::DsInternals
}

#[test]
fn the_moved_sheets_are_consumer_clean() {
    let sheets = own_sheets();
    assert!(sheets.len() > 20, "found {} sheets", sheets.len());
    let rest: Vec<String> = sheets
        .iter()
        .flat_map(|(name, css)| {
            stylesheet(css, &config())
                .into_iter()
                .filter(|offence| !own_class(offence))
                .map(move |offence| {
                    format!(
                        "{name}: {:?} {}:{}: {}",
                        offence.rule, offence.line, offence.column, offence.text
                    )
                })
        })
        .collect();
    assert!(rest.is_empty(), "{}", rest.join("\n"));
}
