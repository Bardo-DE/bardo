//! What the page says and how it looks: no developer words, the provider's mark, legible buttons.

use super::*;

#[test]
fn the_pane_says_no_developer_words_with_the_modules_up_or_gone() {
    let sandbox = Sandbox::new("accounts-plain-no-bus");
    assert_no_developer_words(&sandbox.harness(accounts()));
    let mut harness = Sandbox::new("accounts-plain-none")
        .with_modules(FakeModules::new())
        .harness(accounts());
    settle(&mut harness, "the pane giving up", |h| {
        h.html().contains("The accounts service is not running")
    });
    assert_no_developer_words(&harness);
    let w = world("accounts-plain-up");
    assert_no_developer_words(&w.harness);
}

#[test]
fn the_account_type_is_drawn_with_the_providers_mark() {
    let fake = Accountd::new(
        Verdict::Allow,
        vec![
            Held::new("local-ai", "Ollama", "ok", &["llm"]).of_provider("ollama", "Ollama"),
            Held::new("mail", "Fastmail", "ok", &["mail"]).of_provider("fastmail", "Fastmail"),
        ],
    );
    let mut harness = Sandbox::new("accounts-mark")
        .with_modules(fake.hosted())
        .harness(accounts());
    settle(&mut harness, "the list", |h| h.count(".accounts-list") == 1);
    // The list draws each account's mark, and says the kind of account under its name.
    assert_eq!(
        harness.count(".accounts-list .ds-provider[data-kind=\"face\"]"),
        1
    );
    assert_eq!(
        harness.count(".accounts-list .ds-provider[data-kind=\"letter\"]"),
        1
    );
    assert!(
        harness
            .text_of(".accounts-list")
            .unwrap_or_default()
            .contains("Fastmail")
    );
    click(&mut harness, "#account-local-ai");
    settle(&mut harness, "the account", |h| {
        h.attr(".accounts-detail", "data-account").as_deref() == Some("local-ai")
    });
    assert_eq!(
        harness.count(".accounts-detail .ds-provider[data-kind=\"face\"]"),
        1,
        "a provider whose file gives a face is drawn from it:\n{}",
        harness.html()
    );
    click(&mut harness, "#account-mail");
    settle(&mut harness, "the account", |h| {
        h.attr(".accounts-detail", "data-account").as_deref() == Some("mail")
    });
    assert_eq!(
        harness.count(".accounts-detail .ds-provider[data-kind=\"letter\"]"),
        1,
        "one without a face keeps its named mark"
    );
}

#[test]
fn every_button_says_which_account_or_app_it_is_for() {
    let mut w = world("accounts-names");
    assert_eq!(
        w.harness.count(".ds-row [aria-label^=\"Manage\"]"),
        0,
        "a row is chosen by pressing it, with no button of its own"
    );
    detail(&mut w);
    for (selector, name) in [
        (SIGN_IN, "Sign in again Home cloud"),
        (REMOVE, "Remove account Home cloud"),
    ] {
        assert_eq!(
            w.harness.attr(selector, "aria-label").as_deref(),
            Some(name)
        );
    }
    open(&mut w, KEY);
    assert_eq!(
        w.harness.attr(REVOKE, "aria-label").as_deref(),
        Some("Revoke Photos allowed: llm")
    );
    assert_eq!(
        w.harness.count("[aria-label=\"Revoke\"]"),
        0,
        "no bare Revoke"
    );
}
