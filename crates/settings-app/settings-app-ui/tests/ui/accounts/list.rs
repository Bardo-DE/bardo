//! The list: a row per account, the apps using one, grouping, and what resting on an app says.

use super::*;

#[test]
fn the_list_has_a_row_per_account_of_every_module_with_its_state() {
    let w = world("accounts-list");
    let html = w.harness.html();
    for text in [
        "Home cloud",
        "Work mail",
        "Claude key",
        "Home sync",
        "Signed in",
        "Needs sign-in",
    ] {
        assert!(html.contains(text), "{text}");
    }
    assert_eq!(w.harness.count(".account-row-body"), 4);
    assert!(
        html.contains("Sign-in keys"),
        "the client ids have a disclosure"
    );
    assert_eq!(w.harness.count(CONTINUE_AS), 1);
}

#[test]
fn an_identity_lists_the_apps_using_it_read_only() {
    let mut w = world("accounts-detail");
    detail(&mut w);
    let html = w.harness.html();
    for text in [
        "Apps using this identity",
        "Photos",
        "Mail",
        "Sign in again",
        "Remove account",
    ] {
        assert!(html.contains(text), "{text}");
    }
    assert_eq!(
        w.harness.count("[aria-label^=\"Revoke\"]"),
        0,
        "an identity has no Revoke rows"
    );
    assert_eq!(
        w.harness.count(
            "[aria-label=\"App\"] .ds-field-row-control .ds-button, [aria-label=\"App\"] .ds-toggle"
        ),
        0,
        "the apps are a plain list"
    );
}

#[test]
fn the_list_stays_beside_the_detail_of_the_account_chosen_in_it() {
    let mut w = world("accounts-two-columns");
    assert_eq!(
        w.harness.count(".accounts-detail"),
        0,
        "nothing is chosen yet:\n{}",
        w.harness.html()
    );
    assert!(w.harness.html().contains("No account chosen"));
    // Each choice shows its own account's detail, and the list is there throughout.
    for account in [HOME, KEY, WORK, SYNC] {
        open(&mut w, account);
        assert_eq!(w.harness.count(".accounts-list"), 1, "{}", account.id);
        assert_eq!(w.harness.count(".accounts-detail"), 1, "{}", account.id);
        assert_eq!(
            w.harness.attr(account.row, "aria-selected").as_deref(),
            Some("true"),
            "{} is the chosen row",
            account.id
        );
    }
    // A row is named by its words, read apart, and by its dot when it has one.
    let name = |row: &str| w.harness.attr(row, "aria-label").unwrap_or_default();
    let home = name(HOME.row);
    assert!(home.starts_with("Home cloud, "), "{home}");
    assert_eq!(home.split(", ").count(), 2, "{home}");
    let work = name(WORK.row);
    assert!(work.starts_with("Work mail, "), "{work}");
    assert_eq!(work.split(", ").count(), 3, "the standing is said: {work}");
    // Only an account that is not fine has a dot after its words.
    assert_eq!(
        w.harness
            .count(".account-row-body .ds-badge[data-content=\"dot\"]"),
        1
    );
    assert_eq!(
        w.harness
            .count("#account-work-example .ds-badge[data-content=\"dot\"]"),
        1
    );
}

#[test]
fn resting_the_pointer_on_an_app_names_the_services_it_uses() {
    let mut w = world("accounts-app-card");
    {
        let mut held = w.accountd.accounts.lock().expect("accounts");
        let key = held
            .iter_mut()
            .find(|held| held.id == KEY.id)
            .expect("the key");
        *key = key
            .clone()
            .granting("g3", "Companion can use Language model")
            .granted_on("g3", "2026-10-08", "always")
            .granting("g4", "Companion can use Speech")
    };
    w.announce(
        ModuleId::Accountd,
        "accounts.claude-key.grant.g4",
        toml::Value::String("added".to_owned()),
    );
    open(&mut w, KEY);
    let row = "#permission-accounts-claude-key-grant-g3";
    settle(&mut w.harness, "the permit rows", |h| {
        h.count(row) == 1 && h.html().contains("Allowed on 8 Oct")
    });
    let at = w.harness.centre(row).expect("the row is laid out");
    w.harness.send(Input::pointer_move(at));
    settle(&mut w.harness, "the card", |h| {
        h.html().contains("Uses Language model, Speech")
    });
    assert!(
        w.harness
            .text_of(".ds-hovercard")
            .unwrap_or_default()
            .contains("Companion")
    );
}
