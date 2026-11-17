//! Typed fields: a client id, a stored secret and clearing it.

use super::*;

#[test]
fn a_client_id_is_set_when_the_field_is_left() {
    let mut w = world("accounts-client");
    click(&mut w.harness, "#sign-in-clients .ds-disclosure-trigger");
    settle_on_threads(&mut w.harness, "the field", |h| {
        h.count("[aria-label=\"microsoft client id\"] input") > 0
    });
    click(&mut w.harness, "[aria-label=\"microsoft client id\"] input");
    for letter in "abc".chars() {
        w.harness.send(ui_harness::Input::key(
            ui_components::prelude::ShortcutKey::Char(letter),
        ));
    }
    w.harness.send(ui_harness::Input::key(
        ui_components::prelude::ShortcutKey::Enter,
    ));
    settle_on_threads(&mut w.harness, "the set", |_| !w.accountd.sets().is_empty());
    assert_eq!(
        w.accountd.sets(),
        [(
            "accounts.clients.microsoft".to_owned(),
            toml::Value::String("abc".to_owned())
        )]
    );
}

const SECRET_FIELD: &str = "[aria-label=\"microsoft client secret\"]";

fn type_into(w: &mut World, field: &str, text: &str) {
    click(&mut w.harness, field);
    for letter in text.chars() {
        w.harness.send(ui_harness::Input::key(
            ui_components::prelude::ShortcutKey::Char(letter),
        ));
    }
    w.harness.send(ui_harness::Input::key(
        ui_components::prelude::ShortcutKey::Enter,
    ));
}

#[test]
fn a_stored_secret_shows_set_and_never_its_text_and_typing_replaces_it() {
    let mut w = world("accounts-secret");
    click(&mut w.harness, "#sign-in-clients .ds-disclosure-trigger");
    settle_on_threads(&mut w.harness, "the secret field", |h| {
        h.count(&format!("{SECRET_FIELD} .ds-input-placeholder")) == 1
    });
    assert_eq!(
        w.harness
            .text_of(&format!("{SECRET_FIELD} .ds-input-placeholder"))
            .as_deref(),
        Some("Set"),
        "a held secret says so"
    );
    assert!(
        !w.harness.html().contains(SECRET),
        "the secret's text is nowhere on screen"
    );
    // The secret waits for its key; with the key set, typing replaces the secret.
    type_into(&mut w, "[aria-label=\"microsoft client id\"] input", "abc");
    settle_on_threads(&mut w.harness, "the key to be set", |_| {
        !w.accountd.sets().is_empty()
    });
    settle_on_threads(&mut w.harness, "the secret to open", |h| {
        h.attr(&format!("{SECRET_FIELD} input"), "aria-disabled")
            .as_deref()
            != Some("true")
    });
    type_into(&mut w, &format!("{SECRET_FIELD} input"), "new-secret");
    settle_on_threads(&mut w.harness, "the secret to be sent", |_| {
        w.accountd.sets().len() == 2
    });
    assert_eq!(
        w.accountd.sets()[1],
        (
            "accounts.clients.microsoft.secret".to_owned(),
            toml::Value::String("new-secret".to_owned())
        )
    );
    assert!(!w.harness.html().contains("new-secret"), "nor the new one");
}
