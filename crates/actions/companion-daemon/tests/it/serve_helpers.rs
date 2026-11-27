//! The helpers on `org.bardo.Companion1.Helpers`, `.Helpers2` and `.Terminals` on a private bus:
//! served beside the conversation to the shell alone, with sign-in and routines, and the editor
//! bridge reading them and changing nothing.

use crate::serve::{block, introspect};
use crate::support::world::*;
use account_callers::ProcRoot;
use action_dbus::{
    Bus, COMPANION_PATH, CompanionHelpers2Proxy, CompanionHelpersProxy, CompanionTerminalsProxy,
    introspection,
};
use base_testkit::PrivateBus;
use companion_daemon::{serve_on_rooted, serve_with_routines};
use companion_helpers::RoutineDesk;
use companion_model::helper::{Ceiling, HelperModel, HelperName, Role};
use companion_model::routine::RoutineId;
use companion_tasks::Pending;
use companion_wire::{
    BroughtBack, HelperForm, HelperReply, HelpersView, RoutineCardView, RoutineForm, RoutineReply,
    RoutinesView, TerminalSession,
};
use futures_util::StreamExt;
use futures_util::lock::Mutex;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_helpers_are_served_beside_the_conversation_to_the_shell_alone() {
    let scratch = tempfile::tempdir().expect("scratch");
    let bus = PrivateBus::start_in(scratch.path());
    let (server, shell, stranger) = (
        bus.connect().await,
        bus.connect().await,
        bus.connect().await,
    );
    let mut w = world(vec![]);
    let hub = hub_of(&w);
    w.companion.set_desk(Arc::new(hub));
    let companion = Arc::new(Mutex::new(w.companion));
    let who = who(&server, scratch.path());
    serve_on_rooted(
        &server,
        Arc::clone(&companion),
        Arc::clone(&who),
        ProcRoot::Honoured(scratch.path().to_owned()),
    )
    .await
    .expect("serving");
    as_shell(&who, &shell);

    // Served as declared, on the object the conversation is on.
    let declared = introspection(Bus::Companion);
    let on_bus = introspect(&shell, COMPANION_PATH).await;
    assert_eq!(
        block(&on_bus, "org.bardo.Companion1.Helpers"),
        block(&declared, "org.bardo.Companion1.Helpers")
    );

    // The shell lists none, makes one, and hears that something changed.
    let helpers = CompanionHelpersProxy::new(&shell).await.expect("proxy");
    let mut changed = helpers.receive_changed().await.expect("signal");
    let none: HelpersView =
        serde_json::from_str(&helpers.list(0).await.expect("list")).expect("view");
    assert!(none.helpers.is_empty());
    let form = HelperForm {
        name: HelperName::parse("Research").expect("name"),
        role: Role::parse("Looks things up.").expect("role"),
        space: space("work"),
        model: HelperModel::Unset,
        apps: vec![app("org.bardo.Mail")],
        ceiling: Ceiling::Look,
        steps: base_ids::Count(8),
        kind: companion_model::helper::HelperKind::default(),
    };
    let form = serde_json::to_string(&form).expect("json");
    let made: HelperReply =
        serde_json::from_str(&helpers.create(&form).await.expect("create")).expect("reply");
    let HelperReply::Made(id) = made else {
        panic!("a helper is made: {made:?}")
    };
    tokio::time::timeout(Duration::from_secs(5), changed.next())
        .await
        .expect("the change is told")
        .expect("a signal");
    let listed: HelpersView =
        serde_json::from_str(&helpers.list(0).await.expect("list")).expect("view");
    assert_eq!(listed.helpers.len(), 1);
    assert_eq!(listed.helpers[0].name.as_str(), "Research");

    // Nobody else speaks for the person here: not to read, not to change.
    let id_text = serde_json::to_value(&id)
        .expect("json")
        .as_str()
        .expect("text")
        .to_owned();
    let refused = CompanionHelpersProxy::new(&stranger).await.expect("proxy");
    refused.list(0).await.unwrap_err();
    refused.card(&id_text).await.unwrap_err();
    refused.create(&form).await.unwrap_err();
    assert!(refused.pause(&id_text).await.is_err());
    assert!(refused.remove(&id_text).await.is_err());
    // The shell reads a card, and a helper that is not there is an error, not a guess.
    helpers.card(&id_text).await.unwrap();
    helpers.card("h-404").await.unwrap_err();
}

/// Routines that hold nothing and make nothing: enough to see the calls reach the desk.
#[derive(Debug)]
pub struct EmptyRoutines;

impl RoutineDesk for EmptyRoutines {
    fn view(&self) -> RoutinesView {
        RoutinesView::default()
    }

    fn card(&self, _routine: RoutineId) -> Option<RoutineCardView> {
        None
    }

    fn create(&self, _form: RoutineForm) -> Pending<'_, RoutineReply> {
        Box::pin(async { RoutineReply::Refused("Nothing can be made here.".to_owned()) })
    }

    fn pause(&self, _routine: RoutineId) -> Pending<'_, ()> {
        Box::pin(async {})
    }

    fn resume(&self, _routine: RoutineId) -> Pending<'_, ()> {
        Box::pin(async {})
    }

    fn remove(&self, _routine: RoutineId) -> Pending<'_, ()> {
        Box::pin(async {})
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one scenario told in order; splitting it would hide the sequence being tested"
)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn helpers2_is_served_beside_helpers_with_sign_in_and_routines_to_the_shell_alone() {
    let scratch = tempfile::tempdir().expect("scratch");
    let bus = PrivateBus::start_in(scratch.path());
    let (server, shell, stranger) = (
        bus.connect().await,
        bus.connect().await,
        bus.connect().await,
    );
    let mut w = world(vec![]);
    let hub = hub_of(&w);
    w.companion.set_desk(Arc::new(hub));
    let companion = Arc::new(Mutex::new(w.companion));
    let who = who(&server, scratch.path());
    serve_with_routines(
        &server,
        Arc::clone(&companion),
        Arc::clone(&who),
        ProcRoot::Honoured(scratch.path().to_owned()),
        Some(Arc::new(EmptyRoutines)),
    )
    .await
    .expect("serving");
    as_shell(&who, &shell);

    // Served as declared, and the first version is still there as it was.
    let declared = introspection(Bus::Companion);
    let on_bus = introspect(&shell, COMPANION_PATH).await;
    for name in [
        "org.bardo.Companion1.Helpers",
        "org.bardo.Companion1.Helpers2",
        "org.bardo.Companion1.Terminals",
    ] {
        assert_eq!(block(&on_bus, name), block(&declared, name));
    }

    let helpers = CompanionHelpers2Proxy::new(&shell).await.expect("proxy");
    let none: HelpersView =
        serde_json::from_str(&helpers.list(0).await.expect("list")).expect("view");
    assert!(none.helpers.is_empty());

    // A helper that runs no app has nothing to sign in to or out of, and says so in words.
    let form = HelperForm {
        name: HelperName::parse("Research").expect("name"),
        role: Role::parse("Looks things up.").expect("role"),
        space: space("work"),
        model: HelperModel::Unset,
        apps: vec![app("org.bardo.Mail")],
        ceiling: Ceiling::Look,
        steps: base_ids::Count(8),
        kind: companion_model::helper::HelperKind::default(),
    };
    let form = serde_json::to_string(&form).expect("json");
    let made: HelperReply =
        serde_json::from_str(&helpers.create(&form).await.expect("create")).expect("reply");
    let HelperReply::Made(id) = made else {
        panic!("a helper is made: {made:?}")
    };
    let id_text = serde_json::to_value(&id)
        .expect("json")
        .as_str()
        .expect("text")
        .to_owned();
    for reply in [
        helpers.sign_in(&id_text, "browser").await.expect("sign in"),
        helpers.sign_out(&id_text).await.expect("sign out"),
    ] {
        let reply: HelperReply = serde_json::from_str(&reply).expect("reply");
        assert!(matches!(reply, HelperReply::Refused(_)), "{reply:?}");
    }

    // A helper that runs no app has no conversation to move, and says so in words.
    for reply in [
        helpers.open_in_terminal(&id_text).await.expect("open"),
        helpers.bring_back(&id_text).await.expect("back"),
    ] {
        let reply: HelperReply = serde_json::from_str(&reply).expect("reply");
        assert!(matches!(reply, HelperReply::Refused(_)), "{reply:?}");
    }
    // Conversations a terminal started: none yet, and one that is not there is refused.
    let terminals = CompanionTerminalsProxy::new(&shell).await.expect("proxy");
    let listed: Vec<TerminalSession> =
        serde_json::from_str(&terminals.sessions().await.expect("sessions")).expect("list");
    assert!(listed.is_empty());
    let gone: BroughtBack =
        serde_json::from_str(&terminals.bring_back("s-404").await.expect("bring back"))
            .expect("reply");
    assert!(matches!(gone, BroughtBack::Refused(_)), "{gone:?}");
    terminals.bring_back("not a session").await.unwrap_err();
    // A command line is recognised by the host of the apps, and with none the answer is null.
    let line = serde_json::to_string(&["/usr/bin/claude", "--resume", "ab-1"]).expect("json");
    assert_eq!(terminals.recognise(&line).await.expect("recognise"), "null");
    terminals.recognise("[]").await.unwrap_err();
    terminals.recognise("not json").await.unwrap_err();

    // The routines reach their desk; one that is not there is an error, not a guess.
    let listed: RoutinesView =
        serde_json::from_str(&helpers.routines().await.expect("routines")).expect("view");
    assert!(listed.routines.is_empty());
    helpers.routine_card("r-1").await.unwrap_err();
    helpers.routine_card("not a routine").await.unwrap_err();
    helpers.pause_routine("r-1").await.expect("pause");
    helpers.resume_routine("r-1").await.expect("resume");
    helpers.remove_routine("r-1").await.expect("remove");

    // Nobody else speaks for the person here.
    let refused = CompanionHelpers2Proxy::new(&stranger).await.expect("proxy");
    refused.list(0).await.unwrap_err();
    refused.sign_in(&id_text, "browser").await.unwrap_err();
    refused.routines().await.unwrap_err();
    assert!(refused.remove_routine("r-1").await.is_err());
    refused.open_in_terminal(&id_text).await.unwrap_err();
    refused.bring_back(&id_text).await.unwrap_err();
    let nobody = CompanionTerminalsProxy::new(&stranger)
        .await
        .expect("proxy");
    nobody.sessions().await.unwrap_err();
    nobody.bring_back("s-1").await.unwrap_err();
    nobody.recognise(&line).await.unwrap_err();
}

#[expect(
    clippy::too_many_lines,
    reason = "one scenario told in order; splitting it would hide the sequence being tested"
)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_editor_bridge_reads_the_helpers_and_changes_nothing_and_a_stranger_reads_nothing() {
    let scratch = tempfile::tempdir().expect("scratch");
    let bus = PrivateBus::start_in(scratch.path());
    let (server, shell, editor, stranger) = (
        bus.connect().await,
        bus.connect().await,
        bus.connect().await,
        bus.connect().await,
    );
    let mut w = world(vec![]);
    let hub = hub_of(&w);
    w.companion.set_desk(Arc::new(hub));
    let companion = Arc::new(Mutex::new(w.companion));
    let who = who(&server, scratch.path());
    serve_with_routines(
        &server,
        companion,
        Arc::clone(&who),
        ProcRoot::Honoured(scratch.path().to_owned()),
        Some(Arc::new(EmptyRoutines)),
    )
    .await
    .expect("serving");
    as_shell(&who, &shell);
    who.introduce_app(
        &editor,
        "org.bardo.Acp",
        caller_role_types::CallerRole::Agent,
    );

    let form = HelperForm {
        name: HelperName::parse("Research").expect("name"),
        role: Role::parse("Looks things up.").expect("role"),
        space: space("work"),
        model: HelperModel::Unset,
        apps: vec![app("org.bardo.Mail")],
        ceiling: Ceiling::Look,
        steps: base_ids::Count(8),
        kind: companion_model::helper::HelperKind::default(),
    };
    let form = serde_json::to_string(&form).expect("json");
    let theirs = CompanionHelpers2Proxy::new(&shell).await.expect("proxy");
    let made: HelperReply =
        serde_json::from_str(&theirs.create(&form).await.expect("create")).expect("reply");
    let HelperReply::Made(id) = made else {
        panic!("a helper is made: {made:?}")
    };
    let id_text = serde_json::to_value(&id)
        .expect("json")
        .as_str()
        .expect("text")
        .to_owned();

    // The editor bridge can look, on either version of the interface.
    let helpers = CompanionHelpers2Proxy::new(&editor).await.expect("proxy");
    let listed: HelpersView =
        serde_json::from_str(&helpers.list(0).await.expect("list")).expect("view");
    assert_eq!(listed.helpers.len(), 1);
    helpers.card(&id_text).await.expect("card");
    let first = CompanionHelpersProxy::new(&editor).await.expect("proxy");
    first.list(0).await.expect("list");

    // It changes nothing: every other call is refused in words.
    let refused = |result: zbus::Result<_>| match result {
        Err(zbus::Error::MethodError(name, Some(text), _)) => {
            name.as_str() == "org.freedesktop.DBus.Error.AccessDenied" && text.contains("helpers")
        }
        _ => false,
    };
    assert!(refused(helpers.create(&form).await.map(drop)), "create");
    assert!(
        refused(helpers.edit(&id_text, &form).await.map(drop)),
        "edit"
    );
    assert!(refused(helpers.remove(&id_text).await), "remove");
    assert!(
        refused(helpers.post_job(&id_text, "Do it.", "").await.map(drop)),
        "post job"
    );
    assert!(
        refused(helpers.sign_in(&id_text, "browser").await.map(drop)),
        "sign in"
    );
    assert!(
        refused(helpers.open_in_terminal(&id_text).await.map(drop)),
        "open in terminal"
    );
    assert!(
        refused(helpers.bring_back(&id_text).await.map(drop)),
        "bring back"
    );
    let terminals = CompanionTerminalsProxy::new(&editor).await.expect("proxy");
    assert!(
        refused(terminals.sessions().await.map(drop)),
        "the terminals' conversations"
    );
    assert!(
        refused(terminals.recognise("[\"claude\"]").await.map(drop)),
        "recognise"
    );
    assert!(
        refused(helpers.sign_out(&id_text).await.map(drop)),
        "sign out"
    );
    assert!(refused(helpers.routines().await.map(drop)), "routines");
    assert!(
        refused(helpers.remove_routine("r-1").await),
        "routine write"
    );
    assert!(refused(first.pause(&id_text).await), "pause");

    // A process nothing names reads nothing.
    let nobody = CompanionHelpers2Proxy::new(&stranger).await.expect("proxy");
    nobody.list(0).await.unwrap_err();
    nobody.card(&id_text).await.unwrap_err();

    // The helper is still there.
    let after: HelpersView =
        serde_json::from_str(&theirs.list(0).await.expect("list")).expect("view");
    assert_eq!(after.helpers.len(), 1);
}
