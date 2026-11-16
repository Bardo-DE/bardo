//! The terminal's other surfaces: a dry run goes through the same gate, undo and the journal are
//! the terminal's own rows only, a terminal reaches nothing that is not its to use, and the
//! control centre lists and revokes the standing grants.

use crate::support::*;
use action_core::*;
use action_fake::ScriptedConfirmer;
use action_router::Router;
use action_vocab::{ActionRef, CallRequest, Origin, TargetValue, Value};
use action_wire::{IntentsReply, IntentsRequest, JournalFilter, WireRefusal};
use permission_core::{ActionGrantKey, DenyCode, GrantCaller, GrantTarget};
use provenance::{Actor, UnixSeconds};

fn answering(router: &mut Router<action_fake::FakeSeams>, answers: Vec<ConfirmAnswer>) {
    router.seams.confirmer = ScriptedConfirmer::answering(answers);
}

fn from_terminal() -> ConfirmAnswer {
    ConfirmAnswer::AllowedFromTerminal { receipt: receipt() }
}

fn once() -> ConfirmAnswer {
    ConfirmAnswer::Allowed {
        written: None,
        scope: GrantScope::Once,
        receipt: receipt(),
    }
}

fn shown(router: &Router<action_fake::FakeSeams>) -> Vec<ConfirmRequest> {
    router.seams.confirmer.requests()
}

async fn run(
    router: &Router<action_fake::FakeSeams>,
    request: CallRequest,
) -> Result<Outcome, CallRefusal> {
    match ask(
        router,
        &cli(),
        IntentsRequest::Perform {
            activation: None,
            call: request,
            session: None,
            parent_window: None,
        },
    )
    .await
    {
        IntentsReply::Performed(result) => *result,
        other => panic!("perform: {other:?}"),
    }
}

fn archive() -> CallRequest {
    from_cli("mail.thread.archive", &["t2"], vec![])
}

#[tokio::test]
async fn a_dry_run_goes_through_the_same_gate_and_asks_nobody() {
    let router = router();
    let dry = |call: CallRequest| async {
        ask(
            &router,
            &cli(),
            IntentsRequest::DryRun {
                call,
                session: None,
            },
        )
        .await
    };
    let contact = Value::Entity(provenance::EntityId {
        app: mail_app(),
        kind: provenance::EntityKind::parse("mail.contact").expect("kind"),
        key: provenance::EntityKey::parse("c1").expect("key"),
    });
    let preview = dry(from_cli(
        "mail.message.forward",
        &["t2"],
        vec![("to", contact.clone())],
    ))
    .await;
    assert!(
        matches!(preview, IntentsReply::Preview(Preview::Message { .. })),
        "{preview:?}"
    );
    assert!(shown(&router).is_empty(), "a dry run asks nobody");
    assert!(
        router.seams.link.mail.sent().is_empty(),
        "and sends nothing"
    );
    // No preview declared: still checked, nothing to show.
    let none = dry(from_cli("mail.thread.archive", &["t2"], vec![])).await;
    assert_eq!(none, IntentsReply::Preview(Preview::None));
    // A bad argument is refused exactly as the call would be.
    let bad = dry(from_cli("mail.message.forward", &["t2"], vec![])).await;
    assert!(
        matches!(
            bad,
            IntentsReply::Refused(WireRefusal::Call(CallRefusal::BadArgs { .. }))
        ),
        "{bad:?}"
    );
    // A hidden action is refused, not previewed.
    let hidden = dry(CallRequest {
        action: ActionRef {
            app: app("org.quire.Memory"),
            name: provenance::ActionName::parse("memory.forget").expect("action"),
        },
        target: TargetValue::Entities(vec![provenance::EntityId {
            app: app("org.quire.Memory"),
            kind: provenance::EntityKind::parse("memory.fact").expect("kind"),
            key: provenance::EntityKey::parse("f1").expect("key"),
        }]),
        args: Default::default(),
        origin: Origin::Cli,
    })
    .await;
    assert_eq!(
        hidden,
        IntentsReply::Refused(WireRefusal::Call(CallRefusal::Denied(DenyCode::NotAllowed)))
    );
}

#[tokio::test]
async fn a_terminal_undoes_only_what_a_terminal_did() {
    let mut router = router();
    answering(&mut router, vec![once()]);
    let done = run(&router, archive()).await.expect("archived");
    assert!(matches!(done.undo, Undoable::Journaled(_)));
    // The companion archives another thread.
    let _opened = ready(&router).await;
    perform(&router, call("mail.thread.archive", &["t1"], vec![]))
        .await
        .expect("archived by the companion");
    let mine = match ask(
        &router,
        &cli(),
        IntentsRequest::ControlJournal(JournalFilter {
            run: None,
            session: None,
            limit: account_ids::Count(10),
        }),
    )
    .await
    {
        IntentsReply::Journal(rows) => rows,
        other => panic!("{other:?}"),
    };
    assert_eq!(mine.len(), 1, "a terminal sees only its own rows");
    assert_eq!(mine[0].actor, Actor::Cli);
    let theirs = match ask(
        &router,
        &control(),
        IntentsRequest::ControlJournal(JournalFilter {
            run: None,
            session: None,
            limit: account_ids::Count(10),
        }),
    )
    .await
    {
        IntentsReply::Journal(rows) => rows,
        other => panic!("{other:?}"),
    };
    assert_eq!(theirs.len(), 2);
    let companions = theirs
        .iter()
        .find(|e| e.actor != Actor::Cli)
        .expect("the companion's row");
    assert_eq!(
        ask(&router, &cli(), IntentsRequest::Undo(companions.id)).await,
        IntentsReply::Refused(WireRefusal::NotAllowed)
    );
    assert_eq!(
        ask(&router, &cli(), IntentsRequest::Undo(mine[0].id)).await,
        IntentsReply::Undone(Ok(()))
    );
}

#[tokio::test]
async fn a_terminal_cannot_use_what_is_not_its_to_use() {
    let router = router();
    for request in [
        IntentsRequest::ControlHalt {
            scope: provenance::SpaceScope::Any,
            cause: HaltCause::ControlCentre,
        },
        IntentsRequest::ControlResume {
            scope: provenance::SpaceScope::Any,
        },
        IntentsRequest::ControlTerminalGrants,
        IntentsRequest::ControlTerminalRevoke(action("mail.thread.archive")),
    ] {
        assert_eq!(
            ask(&router, &cli(), request.clone()).await,
            IntentsReply::Refused(WireRefusal::NotAllowed),
            "{request:?}"
        );
    }
}

#[tokio::test]
async fn the_control_centre_lists_and_revokes_the_grants() {
    let mut router = router();
    answering(&mut router, vec![from_terminal()]);
    assert!(run(&router, archive()).await.is_ok());
    assert_eq!(
        ask(&router, &control(), IntentsRequest::ControlTerminalGrants).await,
        IntentsReply::TerminalGrants(vec![action("mail.thread.archive")])
    );
    assert_eq!(
        ask(
            &router,
            &control(),
            IntentsRequest::ControlTerminalRevoke(action("mail.thread.archive"))
        )
        .await,
        IntentsReply::Done
    );
    assert_eq!(
        ask(&router, &control(), IntentsRequest::ControlTerminalGrants).await,
        IntentsReply::TerminalGrants(vec![])
    );
}

#[tokio::test]
async fn a_denial_the_person_recorded_for_the_terminal_stands() {
    use account_core::consent::{Decision, Grant, Usage};
    use action_router::GrantStore;
    let router = router();
    for (n, class) in [provenance::DataClass::Mail].into_iter().enumerate() {
        router.seams.grants.record(Grant {
            id: account_core::GrantId::parse(&format!("g-deny-{n}")).expect("grant"),
            key: ActionGrantKey {
                caller: GrantCaller::Cli,
                owner: mail_app(),
                target: GrantTarget::App,
                class,
                usage: Usage::Interactive,
                space: provenance::SpaceScope::Any,
            },
            decision: Decision::Deny,
            scope: GrantScope::Always,
            at: UnixSeconds(0),
        });
    }
    assert_eq!(
        run(&router, from_cli("mail.thread.read", &["t2"], vec![])).await,
        Err(CallRefusal::Denied(DenyCode::NotAllowed))
    );
}
