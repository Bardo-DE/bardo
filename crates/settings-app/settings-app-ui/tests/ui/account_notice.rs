//! A request from outside to show an account (D-Bus activation's `open-account`): the window
//! moves to that account's detail, or to the list when the account is not known.

use crate::support::Sandbox;
use crate::support::accountd::{Accountd, Held};
use crate::support::drive::page_of;
use settings_app_model::{AccountSegment, AccountTarget, Notice, PageSlug};
use settings_app_ui::Opening;
use ui_harness::Harness;
use ui_settings::live::Verdict;
use std::time::Duration;

fn settle(harness: &mut Harness, what: &str, done: impl Fn(&Harness) -> bool) {
    // The fake daemons answer on real threads; a loaded machine running every test at once can
    // take several seconds, so the budget is generous (15 s) and costs nothing when it settles.
    for _ in 0..750 {
        if done(harness) {
            return;
        }
        harness.advance(Duration::from_millis(50));
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("{what} never happened:\n{}", harness.html());
}

fn detail(segment: &str) -> Notice {
    Notice::OpenAccount(AccountTarget::Detail(AccountSegment(segment.to_owned())))
}

/// The window on another page, over a fake accountd holding two accounts.
fn world(name: &str) -> (Harness, settings_app_model::Outbox, impl Sized) {
    let fake = Accountd::new(
        Verdict::Allow,
        vec![
            Held::new("home", "Home cloud", "ok", &["storage"]),
            Held::new("work.example", "Work mail", "ok", &["mail"]),
        ],
    );
    let opening = Opening {
        page: Some(PageSlug("dock".to_owned())),
        query: String::new(),
    };
    let (harness, outbox) = Sandbox::new(name)
        .with_modules(fake.hosted())
        .harness_told(opening);
    (harness, outbox, fake)
}

#[test]
fn an_account_segment_opens_that_accounts_detail_and_an_unknown_one_the_list() {
    // (the segment, the account whose detail opens; none: the list)
    for (segment, opens) in [("work_example", Some("work.example")), ("nobody", None)] {
        let name = format!("notice-account-{segment}");
        let (mut harness, outbox, _keep) = world(&name);
        outbox.send(detail(segment)).expect("listening");
        settle(&mut harness, "the page", |h| match opens {
            Some(id) => h.attr(".accounts-detail", "data-account").as_deref() == Some(id),
            None => h.count(".accounts-list") == 1,
        });
        assert_eq!(page_of(&harness).as_deref(), Some("accounts"), "{segment}");
    }
}

#[test]
fn the_list_request_leaves_an_open_account() {
    let (mut harness, outbox, _keep) = world("notice-account-list");
    outbox.send(detail("home")).expect("listening");
    settle(&mut harness, "the account", |h| {
        h.count(".accounts-detail") == 1
    });
    outbox
        .send(Notice::OpenAccount(AccountTarget::List))
        .expect("listening");
    settle(&mut harness, "the list", |h| h.count(".accounts-list") == 1);
}

#[test]
fn raising_is_taken_without_a_window_handle_to_ask() {
    let (mut harness, outbox, _keep) = world("notice-raise");
    outbox
        .send(Notice::Raise(Some("token".to_owned())))
        .expect("listening");
    harness.advance(Duration::from_millis(400));
    assert_eq!(page_of(&harness).as_deref(), Some("dock"));
}
