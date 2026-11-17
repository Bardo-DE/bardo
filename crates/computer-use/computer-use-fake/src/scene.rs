//! Ready-made scenes for tests: a window, frames, a world of fakes around it, a task for it, and
//! the replies a script is made of.

use crate::{
    FakeA11y, FakeCapture, FakeDesktop, FakePdp, FakeSeat, FixedClock, Fixture, MemoryEvents,
    ScriptedModel, World,
};
use account_core::AppName;
use base_ids::Px;
use base_time::UnixSeconds;
use computer_use_action::{
    Button, ClickCount, ComputerUseAction, Coord, DeviceSize, FinishOutcome, PixelFormat, Point,
    RunMode, Scale120, Size, Summary, Target, WindowSpace, WindowTrust,
};
use computer_use_bus::{
    ComputerUseTask, ReportRoute, RunBudget, RunOrigin, SuccessCheck, TaskTarget,
};
use computer_use_capture::{Frame, FrameSeq, MaskReport, PixelBuf, WholeMask};
use computer_use_desktop::{
    BackendCaps, BackendKind, MaskWhere, Stamp, ToplevelIdent, ToplevelInfo,
};
use computer_use_run::{ComputerUseConfig, ComputerUseModelError, Rules, Run, RunEnv};
use inference_core::ComputerUseStepReply;
use provenance::{
    Address, AgentRef, Confidentiality, Integrity, Label, Labelled, RunId, SessionId, Source,
    SpaceId,
};
use std::collections::BTreeSet;
use std::sync::Arc;

/// The Space the samples live in.
///
/// # Panics
///
/// Panics if the literal Space id does not parse (a bug).
#[must_use]
pub fn space() -> SpaceId {
    SpaceId::parse("desktop").unwrap()
}

/// The app the sample window belongs to.
///
/// # Panics
///
/// Panics if the literal app name does not parse (a bug).
#[must_use]
pub fn app() -> AppName {
    AppName::parse("org.mozilla.firefox").unwrap()
}

#[must_use]
pub fn frame(seq: u64) -> Frame {
    Frame {
        seq: FrameSeq(seq),
        at: Stamp(seq * 100),
        pixels: PixelBuf {
            format: PixelFormat::Xbgr8888,
            size: DeviceSize { w: 100, h: 80 },
            stride: 400,
            data: Arc::from(vec![u8::try_from(seq % 256).unwrap_or_default(); 400 * 80]),
        },
        logical: Size::new(Coord(100), Coord(80)),
        scale: Scale120(120),
        damage: vec![],
        masks: MaskReport {
            applied_by: MaskWhere::ComputerUseDaemon,
            whole: WholeMask::None,
            regions: vec![],
        },
    }
}

/// A focused native window of [`app`] with a sign-in title.
///
/// # Panics
///
/// Panics if the literal window id does not parse (a bug).
#[must_use]
pub fn window() -> ToplevelInfo {
    ToplevelInfo {
        ident: ToplevelIdent::parse("w1").unwrap(),
        app: app(),
        title: computer_use_desktop::WindowTitle("Sign in".into()),
        pid: computer_use_desktop::Pid(77),
        geometry: computer_use_desktop::GlobalRect {
            x: computer_use_desktop::GlobalCoord(0),
            y: computer_use_desktop::GlobalCoord(0),
            w: Px(100),
            h: Px(80),
        },
        workspace: computer_use_desktop::WorkspaceRef {
            id: "ws-1".into(),
            shown: true,
        },
        // The focused window: input only goes to a window that has the focus on a shared seat.
        state: BTreeSet::from([computer_use_desktop::ToplevelState::Activated]),
        trust: WindowTrust::Native,
        parent: None,
    }
}

/// A model reply that carries just `action`.
///
/// # Errors
///
/// Never returns an error; the `Result` is the shape a scripted reply has.
pub fn reply(
    action: ComputerUseAction<WindowSpace>,
) -> Result<ComputerUseStepReply, ComputerUseModelError> {
    Ok(ComputerUseStepReply::new(vec![action]))
}

#[must_use]
pub fn click() -> ComputerUseAction<WindowSpace> {
    ComputerUseAction::Click {
        at: Target::Point(Point::new(Coord(10), Coord(10))),
        button: Button::Left,
        count: ClickCount::One,
        mods: BTreeSet::new(),
    }
}

/// Ctrl+A: an action that sends keys (the fake keymap has `a`).
///
/// # Panics
///
/// Panics if the literal chord does not parse (a bug).
#[must_use]
pub fn ctrl_a() -> ComputerUseAction<WindowSpace> {
    ComputerUseAction::Key {
        chord: serde_json::from_str(r#"{"mods":["ctrl"],"key":{"Character":"a"}}"#)
            .expect("Ctrl+A"),
        repeat: computer_use_action::Repeat::ONCE,
    }
}

/// An action that finishes the run as done.
///
/// # Panics
///
/// Panics if the literal summary is rejected (a bug).
#[must_use]
pub fn finish() -> ComputerUseAction<WindowSpace> {
    ComputerUseAction::Finish {
        outcome: FinishOutcome::Done,
        summary: Summary::new("Signed in.").unwrap(),
        extracted: vec![],
    }
}

#[must_use]
pub fn user_goal() -> Labelled<String> {
    Labelled::new(
        "sign in".to_owned(),
        Label {
            integrity: Integrity::Trusted,
            confidentiality: Confidentiality::Public,
            classes: BTreeSet::new(),
            sources: BTreeSet::from([Source::User]),
        },
    )
}

pub fn world(
    kind: BackendKind,
    script: Vec<Result<ComputerUseStepReply, ComputerUseModelError>>,
    gate: FakePdp,
) -> World {
    world_over(kind, script, gate, Fixture::FirefoxForm)
}

/// [`world`] over another window's accessibility tree: a test about something other than a
/// sign-in form uses one without a password field.
pub fn world_over(
    kind: BackendKind,
    script: Vec<Result<ComputerUseStepReply, ComputerUseModelError>>,
    gate: FakePdp,
    fixture: Fixture,
) -> World {
    World {
        desktop: FakeDesktop::new(kind, vec![window()]),
        seat: FakeSeat::new(BackendCaps::of(kind).seat, Stamp(0)),
        capture: FakeCapture::scripted((1..=8).map(frame).collect(), vec![]),
        a11y: FakeA11y::fixture(fixture),
        model: ScriptedModel::new(script),
        gate,
        events: MemoryEvents::new(),
        clock: FixedClock::at(Stamp(0), UnixSeconds(1_700_000_000)),
        rules: rules(),
    }
}

/// The screen rules the daemon ships, parsed from the same file.
///
/// # Panics
///
/// Panics if the shipped rules file does not parse (a bug a test catches).
#[must_use]
pub fn rules() -> Rules {
    toml::from_str(include_str!("../../dist/screen-rules.toml"))
        .expect("the shipped screen rules read")
}

/// A run task for the sample window.
///
/// # Panics
///
/// Panics if a literal id does not parse (a bug).
#[must_use]
pub fn task() -> ComputerUseTask {
    ComputerUseTask {
        app: app(),
        goal: user_goal(),
        asked: None,
        earlier: None,
        target: TaskTarget::Window(ToplevelIdent::parse("w1").unwrap()),
        mode: RunMode::InPlace,
        space: space(),
        session: SessionId::parse("s-1").unwrap(),
        conversation: SessionId::parse("s-1").unwrap(),
        budget: RunBudget::PROPOSED,
        data_in: vec![],
        success: SuccessCheck::ModelSays,
        origin: RunOrigin::Planner,
        report: ReportRoute {
            to: Address::new(AgentRef::Companion, space()),
            thread: None,
            in_reply_to: None,
        },
    }
}

/// A run started on `kind`, as `Run::start` leaves it.
///
/// # Panics
///
/// Panics if a literal id does not parse (a bug).
#[must_use]
pub fn started(kind: BackendKind) -> Run {
    let env = RunEnv::new(
        &ComputerUseConfig::default(),
        BackendCaps::of(kind),
        rules(),
    );
    Run::start(RunId::parse("r-1").unwrap(), task(), env, Stamp(0)).0
}
