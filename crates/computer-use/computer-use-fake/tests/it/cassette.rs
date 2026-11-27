//! `ScriptedModel::from_cassette`: recorded turns replay as window-space replies.

use base_ids::Tokens;
use computer_use_action::{Button, ClickCount, ComputerUseAction};
use computer_use_fake::{ScriptError, ScriptedModel};
use computer_use_geometry::{Coord, DeviceSize, PixelFormat, Point, Scale120, Size};
use computer_use_run::{
    ComputerUseModel, ComputerUseModelError, FrameAttachment, StepEvent, StepSink,
};
use inference_core::{DropReason, SafetyHint, StepIndex};
use model_replay::{
    BuildLabel, Cassette, CassetteHeader, CassetteVersion, ContextStamp, EngineLabel, EngineStamp,
    Interaction, InteractionId, RecordedAt, RequestPrint,
};
use model_types::{
    ChoiceScores, EngineExtras, JsonText, Knob, Limits, Message, Milli, ModelName, OutputShape,
    Part, ProviderError, Reasoning, Role, SafetySignal, Sampling, StopReason, ToolCall, ToolCallId,
    ToolChoice, ToolName, ToolParallelism, TurnEnd, TurnEvent, TurnRequest, TurnUsage,
};
use std::collections::BTreeSet;

fn request() -> TurnRequest {
    TurnRequest {
        model: ModelName("holo".into()),
        messages: vec![Message {
            role: Role::User,
            parts: vec![Part::Text("goal".into())],
        }],
        tools: vec![],
        tool_choice: ToolChoice::Auto,
        tool_calls: ToolParallelism::One,
        output: OutputShape::Free,
        limits: Limits {
            max_output: Tokens(64),
            stop: vec![],
            deadline: None,
        },
        sampling: Sampling {
            temperature: Milli(0),
            top_p: Knob::Off,
            top_k: Knob::Off,
            min_p: Knob::Off,
            repeat_penalty: Knob::Off,
            seed: Knob::Off,
        },
        reasoning: Reasoning::Off,
        choice_scores: ChoiceScores::Off,
        engine: EngineExtras::None,
    }
}

fn interaction(id: u32, events: Vec<TurnEvent>, end: Result<(), ProviderError>) -> Interaction {
    let print = RequestPrint::of(&request());
    Interaction {
        id: InteractionId(id),
        print: print.hash(),
        request: print,
        events,
        end: end.map(|()| TurnEnd {
            stop: StopReason::EndTurn,
            usage: TurnUsage::default(),
            served: ModelName("holo".into()),
            first_token: None,
        }),
    }
}

fn cassette(interactions: Vec<Interaction>) -> Cassette {
    Cassette {
        header: CassetteHeader {
            vocab: CassetteVersion::CURRENT,
            engine: EngineStamp {
                kind: EngineLabel("vllm".into()),
                build: BuildLabel("v0".into()),
            },
            model: ModelName("holo".into()),
            recorded: RecordedAt(1),
            context: ContextStamp {
                loaded: Tokens(8192),
                trained: Tokens(32768),
            },
            speech: None,
        },
        interactions,
    }
}

fn tool_call(name: &str, arguments: &str) -> TurnEvent {
    TurnEvent::ToolCallDone(ToolCall {
        id: ToolCallId("call_1".into()),
        name: ToolName::new(name).unwrap(),
        input: JsonText::new(arguments).unwrap(),
    })
}

struct Sink;
impl StepSink for Sink {
    fn event(&mut self, _event: StepEvent) {}
}

fn attachment() -> FrameAttachment {
    FrameAttachment(computer_use_fake::scene::frame(1).pixels)
}

fn blank_request() -> inference_core::ComputerUseStepRequest {
    inference_core::ComputerUseStepRequest::new(
        StepIndex(1),
        inference_core::WindowGeometry {
            logical: Size::new(Coord(1), Coord(1)),
            scale: Scale120(120),
        },
        inference_core::FrameImage {
            source: inference_core::ImageSource::Attached(inference_core::AttachIndex(0)),
            layout: inference_core::FrameLayout::Raw {
                format: PixelFormat::Xbgr8888,
                size: DeviceSize { w: 1, h: 1 },
                stride: 4,
            },
        },
        None,
    )
}

fn left_click(x: u32, y: u32) -> ComputerUseAction<computer_use_geometry::WindowSpace> {
    ComputerUseAction::Click {
        at: computer_use_action::Target::Point(Point::new(Coord(x), Coord(y))),
        button: Button::Left,
        count: ClickCount::One,
        mods: BTreeSet::new(),
    }
}

#[tokio::test]
async fn a_cassette_replays_as_window_space_steps() {
    let c = cassette(vec![
        interaction(
            0,
            vec![
                TurnEvent::ThoughtDelta("look".into()),
                tool_call(
                    "computer_use",
                    r#"{"action":"left_click","coordinate":[5,6]}"#,
                ),
                TurnEvent::Safety(SafetySignal::RequireConfirmation("sure?".into())),
            ],
            Ok(()),
        ),
        interaction(
            1,
            vec![TurnEvent::TextDelta(
                "Thought: next\nAction: click(start_box='(10,20)')\n\nfrobnicate()".into(),
            )],
            Ok(()),
        ),
        interaction(2, vec![], Err(ProviderError::Unreachable)),
        interaction(3, vec![], Err(ProviderError::Refused("no".into()))),
        interaction(4, vec![], Err(ProviderError::Unreadable("x".into()))),
    ]);
    let mut model = ScriptedModel::from_cassette(&c).expect("a script");
    macro_rules! next {
        () => {
            model.step(blank_request(), attachment(), &mut Sink)
        };
    }
    let first = next!().await.expect("first");
    assert_eq!(first.thought.as_deref(), Some("look"));
    assert_eq!(first.actions, vec![left_click(5, 6)]);
    assert_eq!(
        first.safety,
        vec![SafetyHint::RequireConfirmation("sure?".into())]
    );
    let second = next!().await.expect("second");
    assert_eq!(second.thought.as_deref(), Some("next"));
    assert_eq!(second.actions, vec![left_click(10, 20)]);
    assert_eq!(second.dropped.len(), 1);
    assert_eq!(second.dropped[0].verb, "frobnicate");
    assert_eq!(second.dropped[0].reason, DropReason::UnsupportedVerb);
    assert_eq!(next!().await, Err(ComputerUseModelError::Transient));
    assert_eq!(next!().await, Err(ComputerUseModelError::Fatal));
    assert_eq!(next!().await, Err(ComputerUseModelError::Parse));
    // Past the end of the script.
    assert_eq!(next!().await, Err(ComputerUseModelError::Fatal));
}

#[test]
fn a_turn_no_dialect_reads_is_not_a_step() {
    let c = cassette(vec![
        interaction(
            0,
            vec![TurnEvent::TextDelta("Action: wait()".into())],
            Ok(()),
        ),
        interaction(
            1,
            vec![TurnEvent::TextDelta("just chatting".into())],
            Ok(()),
        ),
    ]);
    assert_eq!(
        ScriptedModel::from_cassette(&c).err(),
        Some(ScriptError::NotAStep(1))
    );
}
