use super::*;
use account_capability::ComputerUseEnv;
use base_testkit::Scratch;
use computer_use_action::{Button, ClickCount, ComputerUseAction, Target};
use computer_use_geometry::{Coord, Point, Scale120, Size, WindowSpace};
use inference_core::{
    AttachIndex, FrameImage, FrameLayout, InferEvent, InferRefusedError, MediaType, StepIndex,
    WindowGeometry,
};
use inference_router::testkit::models;
use model_fake::{Script, ScriptedProvider};
use model_types::{
    ModelName, Part, StopReason, ToolCall, ToolCallId, TurnEnd, TurnEvent, TurnUsage,
};
use std::collections::BTreeSet;
use std::io::Write;
use std::ops::ControlFlow;
use std::os::fd::OwnedFd;

fn memfd(bytes: &[u8]) -> OwnedFd {
    let fd = rustix::fs::memfd_create("t", rustix::fs::MemfdFlags::CLOEXEC).expect("memfd");
    std::fs::File::from(fd.try_clone().expect("dup"))
        .write_all(bytes)
        .expect("fill");
    fd
}

fn begin() -> ComputerUseBegin {
    ComputerUseBegin::new("open settings".into(), ComputerUseEnv::Desktop)
}

fn request(step: u32) -> ComputerUseStepRequest {
    ComputerUseStepRequest::new(
        StepIndex(step),
        WindowGeometry {
            logical: Size::new(Coord(200), Coord(100)),
            scale: Scale120(120),
        },
        FrameImage {
            source: inference_core::ImageSource::Attached(AttachIndex(0)),
            layout: FrameLayout::Encoded(MediaType::Png),
        },
        None,
    )
}

fn click(arguments: &str) -> TurnEvent {
    TurnEvent::ToolCallDone(ToolCall {
        id: ToolCallId("c".into()),
        name: model_types::ToolName::new("computer_use").expect("name"),
        input: model_types::JsonText::new(arguments).expect("json"),
    })
}

fn script(events: Vec<TurnEvent>) -> Script {
    Script {
        events,
        end: Ok(TurnEnd {
            stop: StopReason::ToolUse,
            usage: TurnUsage::default(),
            served: ModelName("tiny-computer-use".into()),
            first_token: None,
        }),
    }
}

fn at(x: u32, y: u32) -> ComputerUseAction<WindowSpace> {
    ComputerUseAction::Click {
        at: Target::Point(Point::new(Coord(x), Coord(y))),
        button: Button::Left,
        count: ClickCount::One,
        mods: BTreeSet::default(),
    }
}

/// Collects what a step streams, and stops after `stop_after` events if it says so.
#[derive(Default)]
struct Events {
    seen: Vec<InferEvent>,
    stop_after: Option<usize>,
}

impl ChatSink for Events {
    fn event(&mut self, event: InferEvent) -> ControlFlow<()> {
        self.seen.push(event);
        match self.stop_after {
            Some(n) if self.seen.len() >= n => ControlFlow::Break(()),
            _ => ControlFlow::Continue(()),
        }
    }
}

struct Rig {
    scratch: Scratch,
}

impl Rig {
    fn new() -> Self {
        Self {
            scratch: Scratch::new("computer-use-run"),
        }
    }

    async fn step(
        &self,
        run: &mut ComputerUseRun,
        provider: &ScriptedProvider,
        number: u32,
        sink: &mut Events,
    ) -> Result<ComputerUseStepReply, ComputerUseStepFailure> {
        let model = models(&self.scratch).remove(2);
        let frames = inference_fs::read_frames(vec![memfd(b"png bytes")]).expect("frames");
        let request = request(number);
        run.step(
            StepJob {
                model: &model,
                provider,
                frames: &frames,
                request: &request,
                ceiling: base_ids::Tokens(u32::MAX),
            },
            sink,
        )
        .await
    }
}

#[tokio::test]
async fn a_step_streams_its_thought_and_proposals_into_the_sink_and_returns_the_reply() {
    let provider = ScriptedProvider::new(
        vec![],
        vec![script(vec![
            TurnEvent::TextDelta("clicking".into()),
            click(r#"{"action":"left_click","coordinate":[500,250]}"#),
        ])],
    );
    let mut sink = Events::default();
    let reply = Rig::new()
        .step(&mut ComputerUseRun::begin(begin()), &provider, 0, &mut sink)
        .await
        .expect("a reply");
    assert_eq!(reply.actions, vec![at(100, 25)]);
    assert_eq!(
        sink.seen,
        vec![
            InferEvent::ThoughtDelta("clicking".into()),
            InferEvent::ActionProposed(at(100, 25)),
        ]
    );
}

fn remembered(run: &ComputerUseRun) -> usize {
    run.session
        .as_ref()
        .map_or(0, ComputerUseSession::remembered)
}

fn images(provider: &ScriptedProvider, which: usize) -> usize {
    provider.requests()[which]
        .messages
        .iter()
        .flat_map(|m| m.parts.iter())
        .filter(|part| matches!(part, Part::Image(_)))
        .count()
}

#[tokio::test]
async fn the_run_remembers_its_steps_and_a_step_that_failed_to_reach_the_engine_is_not_one_of_them()
{
    let provider = ScriptedProvider::new(
        vec![],
        vec![
            script(vec![click(
                r#"{"action":"left_click","coordinate":[500,500]}"#,
            )]),
            // The engine fails on the second step.
            Script {
                events: vec![],
                end: Err(model_types::ProviderError::Unreachable),
            },
            script(vec![click(
                r#"{"action":"left_click","coordinate":[100,100]}"#,
            )]),
        ],
    );
    let rig = Rig::new();
    let mut run = ComputerUseRun::begin(begin());
    assert_eq!(remembered(&run), 0);
    let mut sink = Events::default();
    rig.step(&mut run, &provider, 0, &mut sink)
        .await
        .expect("the first");
    assert_eq!(remembered(&run), 1);
    assert_eq!(
        rig.step(&mut run, &provider, 1, &mut sink).await.err(),
        Some(ComputerUseStepFailure::ModelFailed(
            inference_core::ModelError::Unreachable
        ))
    );
    assert_eq!(remembered(&run), 1, "the failed step is not remembered");
    rig.step(&mut run, &provider, 1, &mut sink)
        .await
        .expect("the same step again");
    assert_eq!(remembered(&run), 2);

    // Each prompt holds the frames of the steps before it, up to the model's limit.
    assert_eq!(
        (
            images(&provider, 0),
            images(&provider, 1),
            images(&provider, 2)
        ),
        (1, 2, 2),
        "the failed step's retry sees the same one earlier frame"
    );
    let shown = format!("{run:?}");
    assert!(shown.contains("2 remembered steps"), "{shown}");
    assert!(
        !shown.contains("open settings"),
        "Debug shows sizes, not the goal: {shown}"
    );
}

#[tokio::test]
async fn a_reply_that_never_parses_is_a_step_the_model_took() {
    let provider = ScriptedProvider::new(
        vec![],
        vec![
            script(vec![TurnEvent::TextDelta("hm".into())]),
            script(vec![TurnEvent::TextDelta("hmm".into())]),
        ],
    );
    let mut run = ComputerUseRun::begin(begin());
    let got = Rig::new()
        .step(&mut run, &provider, 0, &mut Events::default())
        .await;
    assert_eq!(got.err(), Some(ComputerUseStepFailure::Unparseable));
    assert_eq!(
        remembered(&run),
        1,
        "listed in the next prompt as a step with no actions"
    );
}

#[tokio::test]
async fn a_model_with_no_computer_use_dialect_cannot_run_a_step() {
    let scratch = Scratch::new("computer-use-chat");
    let chat = models(&scratch).remove(0);
    let provider = ScriptedProvider::new(vec![], vec![]);
    let frames = inference_fs::read_frames(vec![memfd(b"png bytes")]).expect("frames");
    let request = request(0);
    let got = ComputerUseRun::begin(begin())
        .step(
            StepJob {
                model: &chat,
                provider: &provider,
                frames: &frames,
                request: &request,
                ceiling: base_ids::Tokens(u32::MAX),
            },
            &mut Events::default(),
        )
        .await;
    assert_eq!(
        got.err(),
        Some(ComputerUseStepFailure::ModelFailed(
            inference_core::ModelError::Unreadable
        ))
    );
}

#[tokio::test]
async fn a_sink_that_stops_ends_the_proposals_but_not_the_reply() {
    let provider = ScriptedProvider::new(
        vec![],
        vec![script(vec![
            click(r#"{"action":"left_click","coordinate":[500,500]}"#),
            click(r#"{"action":"left_click","coordinate":[100,100]}"#),
        ])],
    );
    let mut sink = Events {
        stop_after: Some(1),
        ..Events::default()
    };
    let reply = Rig::new()
        .step(&mut ComputerUseRun::begin(begin()), &provider, 0, &mut sink)
        .await
        .expect("a reply");
    assert_eq!(sink.seen.len(), 1, "one proposal, then Stop");
    assert_eq!(
        reply.actions.len(),
        2,
        "the reply still holds what was parsed"
    );
}

#[test]
fn only_screen_data_enters_a_computer_use_session() {
    use account_core::DataClass;
    assert_eq!(check_class(DataClass::Screen), Ok(()));
    for class in [
        DataClass::Mail,
        DataClass::Voice,
        DataClass::Public,
        DataClass::AppOwn,
    ] {
        assert_eq!(
            check_class(class),
            Err(InferRefusedError::Unsupported),
            "{class:?}"
        );
    }
}

#[tokio::test]
async fn one_session_serves_every_step_and_a_failed_repair_leaves_its_budget_whole() {
    let provider = ScriptedProvider::new(
        vec![],
        vec![
            script(vec![click(
                r#"{"action":"left_click","coordinate":[500,500]}"#,
            )]),
            // Step 1: a reply that needs a repair, then the engine goes away.
            script(vec![TurnEvent::TextDelta("hm".into())]),
            Script {
                events: vec![],
                end: Err(model_types::ProviderError::Unreachable),
            },
            // The same step again: it may repair again, and does.
            script(vec![TurnEvent::TextDelta("hm".into())]),
            script(vec![click(
                r#"{"action":"left_click","coordinate":[100,100]}"#,
            )]),
            script(vec![click(
                r#"{"action":"left_click","coordinate":[100,100]}"#,
            )]),
        ],
    );
    let rig = Rig::new();
    let mut run = ComputerUseRun::begin(begin());
    let mut sink = Events::default();
    let mut counts = Vec::new();
    for (number, ok) in [(0, true), (1, false), (1, true), (2, true)] {
        let got = rig.step(&mut run, &provider, number, &mut sink).await;
        assert_eq!(got.is_ok(), ok, "step {number}");
        counts.push(remembered(&run));
    }
    assert_eq!(counts, vec![1, 1, 2, 3], "one session grows across steps");
    assert_eq!(provider.requests().len(), 6);
}
