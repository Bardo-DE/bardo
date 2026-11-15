//! AI keys: permits and switches (accountd's rows), and the month's spend (inferd's rows).

use super::*;

#[test]
fn an_ai_key_shows_its_permits_with_revoke_and_revoking_goes_to_accountd() {
    let mut w = world("accounts-permit");
    open(&mut w, KEY);
    let html = w.harness.html();
    assert!(html.contains("Photos allowed: llm"), "{html}");
    assert_eq!(
        w.harness.attr(REVOKE, "aria-label").as_deref(),
        Some("Revoke Photos allowed: llm")
    );
    press_revoke(&mut w);
    settle(&mut w.harness, "the revoke", |_| {
        !w.accountd.sets().is_empty()
    });
    assert_eq!(
        w.accountd.sets(),
        [(
            "accounts.claude-key.grant.g1".to_owned(),
            toml::Value::Boolean(true)
        )]
    );
    assert!(w.syncd.sets().is_empty() && w.inferd.sets().is_empty());
    settle(&mut w.harness, "the permit to leave the page", |h| {
        !h.html().contains("Photos allowed: llm")
    });
}

#[test]
fn a_switch_sets_the_service_in_accountd_and_the_answer_stays_on_screen() {
    let mut w = world("accounts-switch");
    open(&mut w, KEY);
    assert_eq!(w.harness.attr(LLM, "aria-checked").as_deref(), Some("true"));
    click(&mut w.harness, LLM);
    settle(&mut w.harness, "the set", |_| !w.accountd.sets().is_empty());
    assert_eq!(
        w.accountd.sets(),
        [(
            "accounts.claude-key.service.llm".to_owned(),
            toml::Value::String("off".to_owned())
        )]
    );
    settle(&mut w.harness, "the switch to read off", |h| {
        h.attr(LLM, "aria-checked").as_deref() == Some("false")
    });
}
