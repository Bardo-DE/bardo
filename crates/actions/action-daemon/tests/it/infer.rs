//! intentd's models over a scripted inferd session: what is asked (the need, the data class,
//! the tier, the frame), what comes back, and that every way inferd can fail is an error the
//! reviewer turns into asking the person, never a reply.

use crate::support::inferd::*;
use account_capability::Tier;
use account_capability::{LlmFeature, Modality};
use account_core::DataClass;
use account_core::consent::Usage;
use account_ids::Dims;
use account_match::Need;
use account_match::{DimsNeed, EmbedNeed};
use action_core::ReviewTimeouts;
use action_daemon::InferdModel;
use action_review::{ReviewVerdict, Reviewer};
use action_vocab::{Caller, Millis};
use inference_core::{
    ChatControl, ChatMessage, ChatRequest, ChatSink, EmbedRequest, EmbedRole, InferEvent,
    InferRefusal, InferReply, Knob, MessagePart, Model, ModelCard, ModelError, ReplyShape,
    RequestKind, Role, StopReason, ToolChoice, ToolDecl, ToolParallelism, TurnError,
};
use permission_core::{ReviewError, Stage};
use std::collections::BTreeSet;
use std::ops::ControlFlow;

fn card() -> ModelCard {
    ModelCard {
        account: account_core::AccountId::parse("local").expect("account"),
        model: account_capability::ModelId::parse("scripted").expect("model"),
        locality: account_capability::Locality::OnDevice,
        billing: account_capability::Billing::Free,
        capabilities: vec![],
    }
}

fn control() -> ChatControl {
    ChatControl::new()
        .with_tool_choice(ToolChoice::Never)
        .with_tool_calls(ToolParallelism::One)
        .with_max_output(Knob::Off)
        .with_reasoning(inference_core::Reasoning::Off)
        .with_sampling(Knob::Off)
}

fn chat(shape: ReplyShape, tools: Vec<ToolDecl>) -> ChatRequest {
    ChatRequest::new(
        vec![ChatMessage {
            role: Role::User,
            parts: vec![MessagePart::Text("is this fine?".into())],
        }],
        Tier::Balanced,
        DataClass::Prompt,
        Usage::Interactive,
    )
    .with_shape(shape)
    .with_tools(tools)
    .with_control(control())
}

#[derive(Default)]
struct Collect(Vec<InferEvent>);

impl ChatSink for Collect {
    fn event(&mut self, event: InferEvent) -> ControlFlow<()> {
        self.0.push(event);
        ControlFlow::Continue(())
    }
}

fn features_of(need: &Need) -> BTreeSet<LlmFeature> {
    match need {
        Need::Llm(llm) => llm.features.clone(),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn a_chat_opens_one_session_of_the_requests_class_and_tier_and_returns_its_reply() {
    let inferd = ScriptedInferd::answering("pass");
    let model = InferdModel::new(inferd.clone(), card());
    let request = chat(
        ReplyShape::Choice(vec!["pass".into(), "flag".into()]),
        vec![],
    );
    let mut sink = Collect::default();

    let reply = model.chat(&request, &mut sink).await.expect("a reply");

    assert_eq!(reply.text, "pass");
    let opened = inferd.opened();
    assert_eq!(opened.len(), 1, "one session per call");
    assert_eq!(opened[0].class, DataClass::Prompt);
    assert_eq!(opened[0].tier, Tier::Balanced);
    assert_eq!(
        features_of(&opened[0].need),
        BTreeSet::from([LlmFeature::Chat, LlmFeature::StructuredOutput])
    );
    assert_eq!(
        inferd.frames(),
        [inference_core::ClientFrame::Request(
            inference_core::InferRequest::Chat(request)
        )]
    );
    assert!(
        sink.0
            .iter()
            .any(|e| matches!(e, InferEvent::TextDelta(t) if t == "pass")),
        "events reach the sink as they happen: {:?}",
        sink.0
    );
}

#[tokio::test]
async fn the_features_a_chat_needs_follow_its_shape_and_its_tools() {
    let tool = ToolDecl {
        name: inference_core::ToolName::new("archive").expect("tool"),
        description: "Archive".into(),
        params: inference_core::SchemaText(inference_core::JsonText::new("{}").expect("schema")),
    };
    let rows = [
        (
            "plain text",
            ReplyShape::Text,
            vec![],
            vec![LlmFeature::Chat],
        ),
        (
            "json",
            ReplyShape::Json("{}".into()),
            vec![],
            vec![LlmFeature::Chat, LlmFeature::StructuredOutput],
        ),
        (
            "tools",
            ReplyShape::Text,
            vec![tool],
            vec![LlmFeature::Chat, LlmFeature::Tools],
        ),
    ];
    for (name, shape, tools, want) in rows {
        let inferd = ScriptedInferd::answering("ok");
        let model = InferdModel::new(inferd.clone(), card());
        model
            .chat(&chat(shape, tools), &mut Collect::default())
            .await
            .expect("a reply");
        let got = features_of(&inferd.opened()[0].need);
        assert_eq!(got, want.into_iter().collect::<BTreeSet<_>>(), "{name}");
    }
}

#[tokio::test]
async fn every_way_inferd_can_fail_is_a_model_error_and_never_a_reply() {
    let request = chat(ReplyShape::Text, vec![]);
    let finish = |reply: InferReply| ScriptedInferd::new([finishing(RequestKind::Chat, reply)]);
    let rows: Vec<(&str, ScriptedInferd, TurnError)> = vec![
        (
            "nobody home",
            ScriptedInferd::away(),
            TurnError::Failed(ModelError::Unreachable),
        ),
        (
            "this class may not leave the computer",
            finish(InferReply::Refused(InferRefusal::RequiresCloud(
                DataClass::Prompt,
            ))),
            TurnError::Refused(InferRefusal::RequiresCloud(DataClass::Prompt)),
        ),
        (
            "no grant",
            finish(InferReply::Refused(InferRefusal::NeedsGrant)),
            TurnError::Refused(InferRefusal::NeedsGrant),
        ),
        (
            "no model",
            finish(InferReply::Refused(InferRefusal::Unavailable)),
            TurnError::Refused(InferRefusal::Unavailable),
        ),
        (
            "the call failed",
            finish(InferReply::Failed(ModelError::RateLimited(7))),
            TurnError::Failed(ModelError::RateLimited(7)),
        ),
        (
            "cancelled",
            finish(InferReply::Cancelled),
            TurnError::Failed(ModelError::Unreachable),
        ),
        (
            "an answer of another kind",
            finish(InferReply::Embed(inference_core::EmbedReply::new(
                vec![],
                usage(),
                served(),
            ))),
            TurnError::Failed(ModelError::Unparseable),
        ),
        (
            "the session ends with no answer",
            ScriptedInferd::new([inference_core::testing::FakeInferSession::scripted([
                inference_core::testing::Script {
                    kind: RequestKind::Chat,
                    steps: vec![],
                },
            ])]),
            TurnError::Failed(ModelError::Unreachable),
        ),
        (
            "a script for another kind of request",
            ScriptedInferd::new([inference_core::testing::FakeInferSession::scripted([])]),
            TurnError::Refused(InferRefusal::Unsupported),
        ),
    ];
    for (name, inferd, want) in rows {
        let model = InferdModel::new(inferd, card());
        let got = model.chat(&request, &mut Collect::default()).await;
        assert_eq!(got.map(|r| r.text), Err(want), "{name}");
    }
}

#[tokio::test]
async fn embeddings_open_an_embedding_session_for_the_indexs_length() {
    let session = finishing(
        RequestKind::Embed,
        InferReply::Embed(inference_core::EmbedReply::new(
            vec![inference_core::EmbedVector(vec![0.5, 0.25])],
            usage(),
            served(),
        )),
    );
    let inferd = ScriptedInferd::new([session]);
    let model = InferdModel::new(inferd.clone(), card());
    let request = EmbedRequest::new(
        vec!["lisbon".into()],
        EmbedRole::Query,
        DimsNeed::Exactly(Dims(2)),
        DataClass::Notes,
        Usage::Background,
    );

    let reply = model.embed(&request).await.expect("vectors");

    assert_eq!(reply.vectors.len(), 1);
    let opened = inferd.opened();
    assert_eq!(opened[0].class, DataClass::Notes);
    assert_eq!(
        opened[0].need,
        Need::Embeddings(EmbedNeed::new(
            DimsNeed::Exactly(Dims(2)),
            BTreeSet::from([Modality::Text]),
        ))
    );
}

fn review_request() -> action_review::ReviewRequest {
    use action_core::*;
    use action_manifest::Lasting;
    use action_vocab::{LabelText, TurnId};
    use permission_core::{ArgLabels, SessionSaw, Strictness};
    use std::collections::BTreeMap;
    action_review::ReviewRequest {
        space: provenance::SpaceId::parse("work").expect("space"),
        strictness: Strictness::Default,
        turns: vec![UserTurn {
            id: TurnId(1),
            text: "archive the newsletters".into(),
            at: provenance::UnixSeconds(1),
            from: Caller::launcher(),
            via: TurnVia::Typed,
            mode: None,
        }],
        proposed: action_review::ProposedAction {
            app: account_core::AppName::parse("org.quire.Mail").expect("app"),
            action: provenance::ActionName::parse("mail.thread.archive").expect("action"),
            label: LabelText::parse("Archive").expect("label"),
            effect: provenance::Effect::UndoableWrite,
            kinds: BTreeSet::new(),
            count: account_ids::Count(1),
            args: vec![],
            lasting: Lasting::No,
        },
        labels: ArgLabels {
            per_arg: BTreeMap::new(),
            planner: provenance::Integrity::Trusted,
            saw: SessionSaw {
                private: false,
                untrusted: false,
            },
        },
        task_policy: None,
        history: vec![],
    }
}

fn reviewer(quick: ScriptedInferd) -> action_review::InferReviewer<InferdModel<ScriptedInferd>> {
    let model = |inferd| InferdModel::new(inferd, card());
    action_review::InferReviewer {
        quick: model(quick),
        deliberate: model(ScriptedInferd::away()),
        second: model(ScriptedInferd::away()),
        timeouts: ReviewTimeouts {
            quick: Millis(300),
            deliberate: Millis(3000),
            second: Millis(3000),
        },
    }
}

#[tokio::test]
async fn the_reviewer_over_inferd_passes_a_quick_pass_and_asks_when_inferd_says_no() {
    let inferd = ScriptedInferd::answering("pass");
    let verdict = reviewer(inferd.clone())
        .review(Stage::Quick, &review_request())
        .await;
    assert!(matches!(verdict, Ok(ReviewVerdict::Allow)), "{verdict:?}");
    let opened = inferd.opened();
    assert_eq!(
        opened[0].class,
        DataClass::Prompt,
        "a reviewer reads the person's words"
    );
    assert_eq!(opened[0].tier, Tier::Fast);

    for refused in [
        InferRefusal::Unavailable,
        InferRefusal::NeedsGrant,
        InferRefusal::Denied,
    ] {
        let inferd =
            ScriptedInferd::new([finishing(RequestKind::Chat, InferReply::Refused(refused))]);
        let verdict = reviewer(inferd)
            .review(Stage::Quick, &review_request())
            .await;
        assert_eq!(
            verdict.map(|_| ()),
            Err(ReviewError::Unavailable),
            "{refused:?}"
        );
    }
    let verdict = reviewer(ScriptedInferd::away())
        .review(Stage::Quick, &review_request())
        .await;
    assert_eq!(verdict.map(|_| ()), Err(ReviewError::Unavailable));
}

#[tokio::test]
async fn a_reply_cut_short_is_not_a_verdict() {
    let cut = ScriptedInferd::new([inference_core::testing::FakeInferSession::scripted([
        chat_script(reply_stopped("pass", StopReason::MaxTokens)),
    ])]);
    let verdict = reviewer(cut).review(Stage::Quick, &review_request()).await;
    assert_eq!(verdict.map(|_| ()), Err(ReviewError::OutOfRoom));
}
