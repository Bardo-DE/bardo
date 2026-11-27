use super::*;
use account_capability::ComputerUseEnv;
use base_testkit::Scratch;
use computer_use_action::{Button, ClickCount, ComputerUseAction, Target};
use computer_use_geometry::{Coord, Point, Scale120, Size, WindowSpace};
use computer_use_session::TreeText;
use inference_core::{AttachIndex, ImageSource, MediaType, StepIndex};
use inference_router::testkit::models;
use model_fake::{Script, ScriptedProvider};
use model_types::{
    EngineExtras, ModelName, ProviderError, StopReason, ToolCall, ToolCallId as CallId, TurnEnd,
    TurnEvent, TurnUsage,
};
use model_types::{Part, Role, TurnRequest};
use std::collections::BTreeSet;
use std::io::Write;
use std::os::fd::OwnedFd;

fn computer_use_model(scratch: &Scratch) -> LocalModel {
    models(scratch).remove(2)
}

fn memfd(bytes: &[u8]) -> OwnedFd {
    let fd = rustix::fs::memfd_create("t", rustix::fs::MemfdFlags::CLOEXEC).expect("memfd");
    std::fs::File::from(fd.try_clone().expect("dup"))
        .write_all(bytes)
        .expect("fill");
    fd
}

fn begin() -> ComputerUseBegin {
    ComputerUseBegin::new("open settings".into(), ComputerUseEnv::Desktop)
        .with_hints(vec!["it is in the menu".into()])
}

fn request(layout: FrameLayout) -> ComputerUseStepRequest {
    ComputerUseStepRequest::new(
        StepIndex(3),
        inference_core::WindowGeometry {
            logical: Size::new(Coord(200), Coord(100)),
            scale: Scale120(120),
        },
        inference_core::FrameImage {
            source: ImageSource::Attached(AttachIndex(0)),
            layout,
        },
        Some(TreeText("Button: OK".into())),
    )
    .with_prev(vec![PrevResult::Done, PrevResult::Refused("no".into())])
    .with_notes(vec![StepNote {
        from: NoteFrom::Person,
        text: "careful with the red one".into(),
    }])
}

fn png_request() -> ComputerUseStepRequest {
    request(FrameLayout::Encoded(MediaType::Png))
}

fn tool_call(arguments: &str) -> ToolCall {
    ToolCall {
        id: CallId("c1".into()),
        name: model_types::ToolName::new("computer_use").expect("name"),
        input: model_types::JsonText::new(arguments).expect("json"),
    }
}

fn turn_end() -> TurnEnd {
    TurnEnd {
        stop: StopReason::ToolUse,
        usage: TurnUsage::default(),
        served: ModelName("tiny-computer-use".into()),
        first_token: None,
    }
}

fn script(events: Vec<TurnEvent>) -> Script {
    Script {
        events,
        end: Ok(turn_end()),
    }
}

fn click(arguments: &str) -> TurnEvent {
    TurnEvent::ToolCallDone(tool_call(arguments))
}

fn says(events: Vec<TurnEvent>) -> ScriptedProvider {
    ScriptedProvider::new(vec![], vec![script(events)])
}

fn session(model: &LocalModel) -> ComputerUseSession {
    open(model, &begin(), Tokens(u32::MAX)).expect("a session")
}

type Stepped = Result<ComputerUseStepReply, Failed>;

async fn run_one(
    provider: &ScriptedProvider,
    request: &ComputerUseStepRequest,
) -> (Stepped, ComputerUseSession, Vec<InferEvent>) {
    let scratch = Scratch::new("computer-use-step");
    let model = computer_use_model(&scratch);
    let frames = inference_fs::read_frames(vec![memfd(b"png bytes")]).expect("frames");
    let mut seen = Vec::new();
    let mut kept = session(&model);
    let result = step(&mut kept, &model, provider, request, &frames, |event| {
        seen.push(event);
        ControlFlow::Continue(())
    })
    .await;
    (result, kept, seen)
}

fn click_at(x: u32, y: u32) -> ComputerUseAction<WindowSpace> {
    ComputerUseAction::Click {
        at: Target::Point(Point::new(Coord(x), Coord(y))),
        button: Button::Left,
        count: ClickCount::One,
        mods: BTreeSet::default(),
    }
}

fn texts(turn: &TurnRequest, role: Role) -> Vec<String> {
    turn.messages
        .iter()
        .filter(|m| m.role == role)
        .flat_map(|m| m.parts.iter())
        .filter_map(|part| match part {
            Part::Text(text) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn images(turn: &TurnRequest) -> usize {
    turn.messages
        .iter()
        .flat_map(|m| m.parts.iter())
        .filter(|part| matches!(part, Part::Image(_)))
        .count()
}

#[tokio::test]
async fn a_click_on_the_grid_lands_in_window_space_and_is_proposed_then_replied() {
    let provider = says(vec![
        TurnEvent::TextDelta("I will ".into()),
        TurnEvent::TextDelta("click it".into()),
        click(r#"{"action":"left_click","coordinate":[500,250]}"#),
    ]);
    let (result, kept, events) = run_one(&provider, &png_request()).await;
    let reply = result.expect("a reply");
    assert_eq!(reply.actions, vec![click_at(100, 25)]);
    assert_eq!(reply.thought.as_deref(), Some("I will click it"));
    assert_eq!(
        events,
        vec![
            InferEvent::ThoughtDelta("I will ".into()),
            InferEvent::ThoughtDelta("click it".into()),
            InferEvent::ActionProposed(click_at(100, 25)),
        ]
    );
    assert_eq!(kept.remembered(), 1);
}

#[tokio::test]
async fn the_prompt_carries_the_goal_the_results_the_notes_the_tree_and_the_frame() {
    let provider = says(vec![click(r#"{"action":"terminate","status":"success"}"#)]);
    let _ = run_one(&provider, &png_request()).await;
    let sent = &provider.requests()[0];
    assert_eq!(sent.model.0, "tiny-computer-use");
    let system = texts(sent, Role::System).join("\n");
    assert!(system.contains("1000"), "the grid is named: {system}");
    let user = texts(sent, Role::User).join("\n");
    for want in [
        "open settings",
        "it is in the menu",
        "refused (no)",
        "Note: The person says: careful with the red one",
        "Button: OK",
        "Step 3",
    ] {
        assert!(user.contains(want), "{want:?} in {user:?}");
    }
    let last = sent.messages.last().expect("the step's message");
    assert_eq!(last.role, Role::User);
    assert!(
        matches!(last.parts.last(), Some(Part::Image(image)) if image.bytes.0 == b"png bytes"),
        "the frame is last"
    );
}

#[tokio::test]
async fn the_request_is_set_from_the_catalog_entry() {
    let provider = says(vec![click(r#"{"action":"terminate","status":"success"}"#)]);
    let _ = run_one(&provider, &png_request()).await;
    let sent = &provider.requests()[0];
    let scratch = Scratch::new("computer-use-settings");
    let model = computer_use_model(&scratch);
    let entry = model.sampling().expect("sampling");
    assert_eq!(
        sent.sampling, entry.reasoning_off,
        "greedy: the same screen, the same click"
    );
    assert_eq!(sent.reasoning, model_types::Reasoning::Off);
    assert_eq!(
        sent.limits.max_output,
        model.caps().expect("caps").max_output
    );
    assert_eq!(sent.tool_calls, ToolParallelism::One);
    assert_eq!(sent.engine, EngineExtras::None, "vLLM has no extras");
}

#[tokio::test]
async fn a_model_trained_to_think_before_it_acts_is_run_with_its_reasoning() {
    let scratch = Scratch::new("computer-use-think");
    let mut model = computer_use_model(&scratch);
    let Some(ComputerUseSupport::Dialect { profile, .. }) =
        &mut model.entry.capabilities.actions_out
    else {
        panic!("the fixture has a dialect");
    };
    profile.answer = model_types::AnswerShape::ThinkActionCall;
    let provider = says(vec![click(r#"{"action":"terminate","status":"success"}"#)]);
    let frames = inference_fs::read_frames(vec![memfd(b"png bytes")]).expect("frames");
    let mut kept = session(&model);
    let _ = step(
        &mut kept,
        &model,
        &provider,
        &png_request(),
        &frames,
        |_| ControlFlow::Continue(()),
    )
    .await;
    let sent = &provider.requests()[0];
    let entry = model.sampling().expect("sampling");
    assert_eq!(sent.reasoning, model_types::Reasoning::EngineDefault);
    assert_eq!(
        sent.sampling,
        *entry.for_reasoning(model_types::Reasoning::EngineDefault)
    );
}

#[tokio::test]
async fn an_engine_failure_is_told_as_such_and_the_step_does_not_count() {
    let down = ScriptedProvider::new(
        vec![],
        vec![Script {
            events: vec![],
            end: Err(ProviderError::Unreachable),
        }],
    );
    let (result, kept, _) = run_one(&down, &png_request()).await;
    let Err(Failed { failure }) = result else {
        panic!("a failure");
    };
    assert_eq!(
        failure,
        ComputerUseStepFailure::ModelFailed(ModelError::Unreachable)
    );
    assert_eq!(kept.remembered(), 0);
}

#[tokio::test]
async fn the_prompt_holds_one_frame_fewer_than_the_model_takes_and_lists_the_steps() {
    let scratch = Scratch::new("computer-use-history");
    let model = computer_use_model(&scratch);
    let per_prompt = model.caps().expect("caps").images.per_prompt.0;
    assert_eq!(per_prompt, 3);
    let scripts = (0..5)
        .map(|_| {
            script(vec![click(
                r#"{"action":"left_click","coordinate":[500,500]}"#,
            )])
        })
        .collect();
    let provider = ScriptedProvider::new(vec![], scripts);
    let mut current = session(&model);
    for _ in 0..5 {
        let frames = inference_fs::read_frames(vec![memfd(b"png")]).expect("frames");
        step(
            &mut current,
            &model,
            &provider,
            &png_request(),
            &frames,
            |_| ControlFlow::Continue(()),
        )
        .await
        .expect("a step");
    }
    let requests = provider.requests();
    let held: Vec<usize> = requests.iter().map(images).collect();
    assert_eq!(
        held,
        vec![1, 2, 3, 3, 3],
        "the images of a prompt never pass {per_prompt}"
    );
    assert_eq!(current.remembered(), 5);
}

#[tokio::test]
async fn a_stop_from_the_sink_ends_the_turn_so_there_is_nothing_to_act_on_and_no_repair() {
    let provider = ScriptedProvider::new(
        vec![],
        vec![
            script(vec![
                TurnEvent::TextDelta("a".into()),
                TurnEvent::TextDelta("b".into()),
                click(r#"{"action":"left_click","coordinate":[500,500]}"#),
            ]),
            script(vec![click(
                r#"{"action":"left_click","coordinate":[500,500]}"#,
            )]),
        ],
    );
    let scratch = Scratch::new("computer-use-stop");
    let model = computer_use_model(&scratch);
    let frames = inference_fs::read_frames(vec![memfd(b"png")]).expect("frames");
    let mut count = 0;
    let mut kept = session(&model);
    let result = step(
        &mut kept,
        &model,
        &provider,
        &png_request(),
        &frames,
        |_| {
            count += 1;
            ControlFlow::Break(())
        },
    )
    .await;
    assert_eq!(count, 1, "the first stop ends the pushing");
    assert_eq!(
        result.err().map(|failed| failed.failure),
        Some(ComputerUseStepFailure::Unparseable)
    );
    assert_eq!(provider.requests().len(), 1, "nobody is asked for a repair");
}

#[test]
fn a_session_opens_for_a_tool_or_text_dialect_and_for_nothing_else() {
    let scratch = Scratch::new("computer-use-open");
    let mut models = models(&scratch);
    assert!(open(&models[2], &begin(), Tokens(u32::MAX)).is_some());
    assert!(
        open(&models[0], &begin(), Tokens(u32::MAX)).is_none(),
        "a chat model has no dialect"
    );
    models[2].entry.capabilities.actions_out = Some(ComputerUseSupport::Dialect {
        dialect: ComputerUseDialect::Wire(model_types::WireDialect::OpenAiComputer),
        batching: Batching::One,
        zoom: model_types::Zoom::Absent,
        profile: model_types::ActionProfile::default(),
    });
    assert!(open(&models[2], &begin(), Tokens(u32::MAX)).is_none());
}

#[test]
fn what_became_of_the_actions_is_told_in_the_model_layers_three_words() {
    let id = |n: usize| CallId(format!("prev-{n}"));
    let got = prev_results(&[
        PrevResult::Done,
        PrevResult::Refused("no".into()),
        PrevResult::NotRun,
        PrevResult::Failed("gone".into()),
        PrevResult::UserDeclined,
        PrevResult::UserActed,
    ]);
    assert_eq!(
        got,
        vec![
            StepResult::Done(id(0)),
            StepResult::Refused {
                id: id(1),
                why: "no".into()
            },
            StepResult::NotRun(id(2)),
            StepResult::Refused {
                id: id(3),
                why: "gone".into()
            },
            StepResult::Refused {
                id: id(4),
                why: "the person declined it".into()
            },
            StepResult::Refused {
                id: id(5),
                why: "the person did it themselves".into()
            },
        ]
    );
}

#[test]
fn the_observation_carries_the_window_contents_and_the_notes_with_who_said_them() {
    let seen = observation(&png_request());
    assert_eq!(seen.step, StepIndex(3));
    assert_eq!(
        seen.tree.as_ref().map(|tree| tree.0.as_str()),
        Some("Button: OK")
    );
    assert_eq!(
        seen.notes,
        vec![NoteLine("The person says: careful with the red one".into())]
    );
    let mut helper = png_request();
    helper.notes = vec![StepNote {
        from: NoteFrom::Agent,
        text: "a dialog opened".into(),
    }];
    helper.tree = None;
    let seen = observation(&helper);
    assert_eq!(seen.tree, None);
    assert_eq!(
        seen.notes,
        vec![NoteLine("A helper says: a dialog opened".into())]
    );
}

#[tokio::test]
async fn an_engine_failure_after_a_repair_gives_the_repair_budget_back() {
    let scratch = Scratch::new("computer-use-refill");
    let model = computer_use_model(&scratch);
    let frames = inference_fs::read_frames(vec![memfd(b"png bytes")]).expect("frames");
    let provider = ScriptedProvider::new(
        vec![],
        vec![
            script(vec![TurnEvent::TextDelta("hm".into())]),
            Script {
                events: vec![],
                end: Err(ProviderError::Unreachable),
            },
        ],
    );
    let mut kept = session(&model);
    let full = kept.repairs_left();
    let result = step(
        &mut kept,
        &model,
        &provider,
        &png_request(),
        &frames,
        |_| ControlFlow::Continue(()),
    )
    .await;
    assert!(matches!(
        result,
        Err(Failed {
            failure: ComputerUseStepFailure::ModelFailed(ModelError::Unreachable)
        })
    ));
    assert_eq!(provider.requests().len(), 2, "the turn, then the repair");
    assert_eq!(kept.repairs_left(), full, "the same step can repair again");
    assert_eq!(kept.remembered(), 0, "the step does not count");
}
