//! The screen-sharing picker's host side on a private bus: a stand-in transom (the owner of the
//! portal backend's name) calls `PickCaptureSource` on the connection the seat holds; the shell's
//! own part is stepped through `Answer`. Only the portal backend may ask (who counts as it is
//! `callers_bus.rs`'s), and a second call while one is open is refused. `PickColor` is the same:
//! one question at a time, an empty array while nothing draws the eyedropper, the colour once it
//! is clicked. A test skips, saying so, without a `dbus-daemon`. Never the person's session bus.

use std::time::Duration;

use desktop_protocols::vocab::{
    Emphasis, LeaseState, Modality, PlacementMark, Responsiveness, SensitivityLevel, WindowRole,
    WindowStateKind,
};
use shell_model::apps::AppIndex;
use shell_model::capture_pick::{
    CaptureCommand, CaptureLink, CapturePicker, Picks, Plurality, Rgb8, Source, plan, toggle,
};
use shell_model::desktop::{DesktopCommand, DesktopView, SpacePlaceView, WindowView};
use shell_model::displays::Displays;
use shell_services::capture_pick::{CaptureLinks, CaptureService};
use shell_services::env::Env;
use shell_services::seat::seat;
use shell_services::service::spawn;
use shell_testkit::{PrivateBus, TempDir};
use tokio::sync::watch;
use zbus::Connection;
use zbus::connection::Builder;

use crate::callers_support::{Introduced, Role, Who};

const SETTLE: Duration = Duration::from_secs(5);
const PORTAL: &str = "org.freedesktop.impl.portal.desktop.bardo";

#[zbus::proxy(
    interface = "org.bardo.Shell1",
    default_service = "org.bardo.Shell",
    default_path = "/org/bardo/Shell1",
    gen_blocking = false
)]
pub(crate) trait Shell1 {
    fn pick_capture_source(
        &self,
        app_label: &str,
        kinds: u32,
        multiple: u32,
        cursor_modes: Vec<u32>,
        remembered: u32,
    ) -> zbus::Result<(Vec<(String, String)>, u32)>;

    fn pick_color(&self, app_label: &str) -> zbus::Result<Vec<(f64, f64, f64)>>;

    fn ask_permission(
        &self,
        app_label: &str,
        title: &str,
        subtitle: &str,
        body: &str,
        grant_label: &str,
        deny_label: &str,
    ) -> zbus::Result<u32>;
}

pub(crate) async fn wait_for<T>(
    receiver: &mut watch::Receiver<T>,
    want: impl Fn(&T) -> bool,
) -> bool {
    tokio::time::timeout(SETTLE, receiver.wait_for(|value| want(value)))
        .await
        .is_ok_and(|seen| seen.is_ok())
}

pub(crate) async fn connect(bus: &PrivateBus, name: Option<&str>) -> Connection {
    let builder = Builder::address(bus.address.as_str())
        .unwrap_or_else(|e| panic!("{e}"))
        // A call the other end never answers fails the test instead of hanging it.
        .method_timeout(std::time::Duration::from_secs(8));
    let builder = match name {
        Some(name) => builder
            .name(name.to_owned())
            .unwrap_or_else(|e| panic!("{e}")),
        None => builder,
    };
    builder
        .build()
        .await
        .unwrap_or_else(|e| panic!("connecting: {e}"))
}

/// The desktop with one ordinary window, which the picker offers by its identifier.
fn desktop_with_notes() -> DesktopView {
    let notes = WindowView {
        identifier: "w-1".to_owned(),
        app: "Notes".to_owned(),
        title: "Notes".to_owned(),
        state: WindowStateKind::Floating,
        space: SpacePlaceView::Unknown,
        output: String::new(),
        emphasis: Emphasis::Inactive,
        placement: PlacementMark::Free,
        role: WindowRole::Document,
        modality: Modality::None,
        geometry: None,
        responsiveness: Responsiveness::Responding,
        lease: LeaseState::Free,
        sensitivity: SensitivityLevel::Normal,
    };
    DesktopView {
        windows: vec![notes],
        ..DesktopView::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn transom_asks_the_person_picks_and_the_answer_comes_back() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let scratch = TempDir::new("capture-pick-bus");
    let Rig {
        mut service,
        transom,
        desktop,
        ..
    } = rig(&bus, &scratch).await;
    desktop.send_replace(desktop_with_notes());
    assert!(
        wait_for(&mut service.state, |s: &CapturePicker| {
            !s.candidates.windows.is_empty()
        })
        .await,
        "the desktop's window was not offered"
    );

    let proxy = Shell1Proxy::new(&transom)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let first = {
        let proxy = proxy.clone();
        tokio::spawn(async move { proxy.pick_capture_source("Meet", 2, 0, vec![1, 2], 0).await })
    };
    assert!(
        wait_for(&mut service.state, |s: &CapturePicker| s.asking.is_some()).await,
        "the sheet was not opened"
    );
    // One at a time: the second call is refused at once.
    proxy
        .pick_capture_source("Other", 1, 0, vec![1], 0)
        .await
        .unwrap_err();

    let (id, ask, candidates) = {
        let state = service.state.borrow();
        let asking = state.asking.clone().unwrap_or_else(|| panic!("open"));
        (asking.id, asking.ask, state.candidates.clone())
    };
    let plan = plan(&ask, &candidates);
    let picks = toggle(
        &plan,
        Plurality::One,
        Picks::start(&plan),
        Source::Window("w-1".to_owned()),
    );
    let _ = service
        .control
        .send(CaptureCommand::Answer(id, Some(picks)));
    let answer = first
        .await
        .unwrap_or_else(|e| panic!("{e}"))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(answer, (vec![("window".to_owned(), "w-1".to_owned())], 1));
}

/// The service serving on a private bus, with a stand-in transom connected.
pub(crate) async fn serving(
    bus: &PrivateBus,
    scratch: &TempDir,
) -> (shell_services::service::Handle<CaptureService>, Connection) {
    let rig = rig(bus, scratch).await;
    (rig.service, rig.transom)
}

/// The service serving on a private bus: what stands in for transom, for the shell's seat, and
/// for the compositor (the answers it hears).
pub(crate) struct Rig {
    pub service: shell_services::service::Handle<CaptureService>,
    /// The portal backend: the owner of its name, introduced as the program that is.
    pub transom: Connection,
    /// What the compositor is told the person answered.
    pub answers: tokio::sync::mpsc::UnboundedReceiver<DesktopCommand>,
    /// Who the stand-ins are, to the shell.
    pub callers: std::sync::Arc<Introduced>,
    /// Where the desktop's view is written, as the windows service does.
    pub desktop: watch::Sender<DesktopView>,
}

pub(crate) async fn rig(bus: &PrivateBus, scratch: &TempDir) -> Rig {
    rig_as(bus, scratch, Who::Service(Role::Portal)).await
}

/// As [`rig`], the owner of the portal backend's name being `who` to the shell.
pub(crate) async fn rig_as(bus: &PrivateBus, scratch: &TempDir, who: Who) -> Rig {
    let callers = Introduced::new();
    let env = Env {
        callers: std::sync::Arc::<Introduced>::clone(&callers),
        ..Env::isolated(scratch.path())
    };
    let transom = connect(bus, Some(PORTAL)).await;
    callers.make(&transom, who);
    let shell = connect(bus, Some("org.bardo.Shell")).await;
    // The shell's connection already serves objects when the picker exports its own; without
    // this, a look sent before zbus's object server runs is dropped unanswered (see dbus-kit's ready).
    dbus_kit::ready::serving(&shell)
        .await
        .unwrap_or_else(|e| panic!("the shell's connection takes calls: {e}"));
    let (publisher, view) = seat();
    publisher.publish(shell);
    let (_displays_tx, displays) = watch::channel(Displays::default());
    let (desktop, desktop_view) = watch::channel(DesktopView::default());
    let (answers_tx, answers) = tokio::sync::mpsc::unbounded_channel();
    let mut service = spawn::<CaptureService>(
        &tokio::runtime::Handle::current(),
        &env,
        CaptureLinks {
            seat: view,
            displays,
            desktop: desktop_view,
            apps: watch::channel(AppIndex::default()).1,
            answers: answers_tx,
        },
    );
    let _ = service.control.send(CaptureCommand::Serve);
    assert!(
        wait_for(&mut service.state, |s: &CapturePicker| s.link
            == CaptureLink::Serving)
        .await
    );
    // The object is exported a moment after the link says serving, and nothing announces it:
    // look for it by introspection (each look answered at once now), up to the generous deadline.
    let deadline = base_testkit::Deadline::generous();
    loop {
        let introspect = zbus::fdo::IntrospectableProxy::builder(&transom)
            .destination("org.bardo.Shell")
            .and_then(|builder| builder.path("/org/bardo/Shell1"))
            .unwrap_or_else(|e| panic!("{e}"))
            .build()
            .await
            .unwrap_or_else(|e| panic!("{e}"))
            .introspect()
            .await;
        if introspect.is_ok_and(|xml| xml.contains("org.bardo.Shell1")) {
            break;
        }
        assert!(!deadline.passed(), "the picker object was never exported");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    Rig {
        service,
        transom,
        answers,
        callers,
        desktop,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_colour_request_is_cancelled_at_once_while_nothing_draws_the_eyedropper() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let scratch = TempDir::new("capture-pick-colour-absent");
    let (service, transom) = serving(&bus, &scratch).await;
    let proxy = Shell1Proxy::new(&transom)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let answer = proxy
        .pick_color("Screenshot")
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(answer, Vec::<(f64, f64, f64)>::new());
    assert!(service.state.borrow().color.is_none());
}

/// Tell the service a surface draws the eyedropper and wait until it has taken that in. The
/// service reads its commands and the bus calls on two channels and takes whichever is ready, so
/// a call made straight after the command can be handled first and cancelled at once.
async fn draw_eyedropper(service: &mut shell_services::service::Handle<CaptureService>) {
    let _ = service.control.send(CaptureCommand::Eyedropper(true));
    assert!(
        wait_for(&mut service.state, |s: &CapturePicker| s.eyedropper).await,
        "the eyedropper was not taken in"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_colour_is_asked_one_at_a_time_and_the_click_comes_back_as_floats() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let scratch = TempDir::new("capture-pick-colour");
    let (mut service, transom) = serving(&bus, &scratch).await;
    draw_eyedropper(&mut service).await;
    let proxy = Shell1Proxy::new(&transom)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let first = {
        let proxy = proxy.clone();
        tokio::spawn(async move { proxy.pick_color("Screenshot").await })
    };
    assert!(
        wait_for(&mut service.state, |s: &CapturePicker| s.color.is_some()).await,
        "the eyedropper was not opened"
    );
    // One at a time, of either kind: both second calls are refused at once.
    proxy.pick_color("Other").await.unwrap_err();
    proxy
        .pick_capture_source("Other", 1, 0, vec![1], 0)
        .await
        .unwrap_err();
    let id = service
        .state
        .borrow()
        .color
        .as_ref()
        .map_or_else(|| panic!("open"), |asking| asking.id);
    let _ = service.control.send(CaptureCommand::ColorAnswer(
        id,
        Some(Rgb8 {
            red: 255,
            green: 0,
            blue: 51,
        }),
    ));
    let answer = first
        .await
        .unwrap_or_else(|e| panic!("{e}"))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(answer, vec![(1.0, 0.0, 51.0 / 255.0)]);
    // The answer can reach the caller before the service records that the eyedropper is free.
    assert!(
        wait_for(&mut service.state, |s: &CapturePicker| s.color.is_none()).await,
        "the eyedropper is free again"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_colour_request_from_anyone_but_the_portal_backend_is_refused() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let scratch = TempDir::new("capture-pick-colour-forged");
    let (mut service, _transom) = serving(&bus, &scratch).await;
    draw_eyedropper(&mut service).await;
    let forger = connect(&bus, None).await;
    let proxy = Shell1Proxy::new(&forger)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    proxy.pick_color("Evil").await.unwrap_err();
    assert!(service.state.borrow().color.is_none());
}
