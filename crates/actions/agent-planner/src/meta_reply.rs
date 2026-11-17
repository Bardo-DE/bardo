//! Reading a reply's words and its meta calls (`bardo_answer`, `bardo_ask`, `bardo_read`,
//! `bardo_finish`, `bardo_related`) into the loop's output, and the limits on what a model's
//! words may carry.

use crate::plain::without_ids;
use crate::planner::{
    PlanError, TOOL_ANSWER, TOOL_ASK, TOOL_FINISH, TOOL_READ, TOOL_RELATED, log_unreadable,
};
use crate::read_ask::read_output;
use account_core::AppName;
use action_core::{HandleShape, OneOf, PlannerView, ReplyFault};
use action_manifest::{ArgError, ArgsError, RELATION_ARG};
use action_vocab::{Args, CallRequest, Handle, Origin, ParamName, TargetValue, Value};
use agent_loop::{Availability, ModelOutput, PlannedCall, choose_tier, leaked_call};
use inference_core::ToolCall;
use provenance::{EntityId, EntityKey, EntityKind};
use serde_json::Value as Json;

/// The first argument of `json` that names a value outside what this turn allows it.
pub(crate) fn outside_known(known: &[OneOf], json: &Json) -> Option<ReplyFault> {
    known.iter().find_map(|one_of| {
        let given = json.get(one_of.param.as_str())?.as_str()?;
        (!one_of.admits(given)).then(|| ReplyFault::NotOneOf {
            param: one_of.param.clone(),
            options: one_of.values.clone(),
        })
    })
}

/// A name the model wrote, as far as it is plain enough to say back: letters, digits and `._-`,
/// at most 64 of them; anything else is not repeated.
pub(crate) fn plain_name(name: &str) -> String {
    let plain = name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if plain {
        name.to_owned()
    } else {
        String::new()
    }
}

/// An argument fault that names nothing the model wrote but a plain name.
pub(crate) fn plain_fault(fault: ArgsError) -> ArgsError {
    match fault {
        ArgsError::Unknown(name) => ArgsError::Unknown(plain_name(&name)),
        other => other,
    }
}

/// The most words of one reply that are kept: a model with no output limit can say a great deal,
/// and the person's answer, the history and the bus carry what is kept.
pub(crate) const MOST_WORDS: usize = 32_000;

/// `text` without the marks that reorder or hide words: the model's words are drawn as written,
/// never reversed or concealed.
fn plain(text: &str) -> String {
    text.chars()
        .filter(|c| !action_vocab::reorders(*c) && !action_vocab::hides(*c))
        .collect()
}

/// `text` as the person is shown it: plain, and cut at [`MOST_WORDS`] characters with an
/// ellipsis where it was cut.
pub(crate) fn bounded(text: &str) -> String {
    let shown = plain(text);
    match shown.char_indices().nth(MOST_WORDS) {
        Some((end, _)) => format!("{}\u{2026}", &shown[..end]),
        None => shown,
    }
}

pub(crate) fn is_meta(name: &str) -> bool {
    matches!(
        name,
        TOOL_ANSWER | TOOL_ASK | TOOL_READ | TOOL_FINISH | TOOL_RELATED
    )
}

/// The words of a `bardo_answer`: plain, not empty, and not a call written as words. A reply
/// that is none of these is unreadable, like a question that is none of them.
pub(crate) fn answer_text(call: &ToolCall) -> Result<String, PlanError> {
    let args: Json = serde_json::from_str(call.input.as_str()).map_err(|error| {
        log_unreadable("an answer's arguments", &error);
        PlanError::Unreadable
    })?;
    let text = args
        .get("text")
        .and_then(Json::as_str)
        .map(|t| bounded(t.trim()))
        .filter(|t| !t.is_empty())
        .ok_or(PlanError::Unreadable)?;
    match leaked_call(&text) {
        Some(_) => Err(PlanError::CallInText),
        None => Ok(without_ids(&text)),
    }
}

pub(crate) fn meta_output(call: &ToolCall, view: &PlannerView) -> Result<ModelOutput, PlanError> {
    let args = match serde_json::from_str::<Json>(call.input.as_str()) {
        Ok(args) => args,
        Err(error) => {
            log_unreadable("a meta call's arguments", &error);
            return match call.name.as_str() {
                TOOL_READ | TOOL_RELATED => Ok(ModelOutput::Unread(ReplyFault::NotJson)),
                _ => Err(PlanError::Unreadable),
            };
        }
    };
    match call.name.as_str() {
        TOOL_FINISH => Ok(ModelOutput::Finish),
        TOOL_ASK => question(&args),
        TOOL_RELATED => Ok(related_output(&args, view)),
        _ => Ok(read_output(&args)),
    }
}

fn param(name: &str) -> Option<ParamName> {
    ParamName::parse(name).ok()
}

fn wrong(name: &str, why: ArgError) -> ModelOutput {
    match param(name) {
        Some(param) => ModelOutput::Unread(ReplyFault::Args(ArgsError::Wrong { param, why })),
        None => ModelOutput::Unread(ReplyFault::NotJson),
    }
}

fn missing(name: &str) -> ModelOutput {
    match param(name) {
        Some(param) => ModelOutput::Unread(ReplyFault::Args(ArgsError::Missing(param))),
        None => ModelOutput::Unread(ReplyFault::NotJson),
    }
}

/// A `bardo_related` as the call of the related action of the thing's kind: the same call the
/// router gates and records as any other read, so a planner gets a related thing no other way
/// than through the gate. The thing is a handle the view shows or a thing named as `target`
/// names it (`{"app", "kind", "key"}`, as the context shows the thing the person has open), and
/// its kind must have that relation.
fn related_output(args: &Json, view: &PlannerView) -> ModelOutput {
    let Some(of) = args.get("of") else {
        return missing("of");
    };
    let Some(relation) = args.get("relation").and_then(Json::as_str) else {
        return missing(RELATION_ARG);
    };
    let Some((target, kind)) = related_subject(of, view) else {
        return wrong("of", ArgError::Type);
    };
    let kind = &kind;
    let Some(card) = view.actions.iter().find(|card| {
        matches!(&card.on, action_manifest::TargetKind::One(k) if k == kind)
            && card.related.iter().any(|r| r.name.as_str() == relation)
    }) else {
        return wrong(RELATION_ARG, ArgError::Range);
    };
    let (Some(name), Some(choice)) = (
        param(RELATION_ARG),
        action_vocab::ChoiceId::parse(relation).ok(),
    ) else {
        return wrong(RELATION_ARG, ArgError::Range);
    };
    let Some(tier) = choose_tier(&Availability {
        typed: true,
        hook: false,
        computer_use: false,
    }) else {
        return ModelOutput::Unread(ReplyFault::NoSuchTool(String::new()));
    };
    let mut args = Args::new();
    args.insert(
        name,
        provenance::Labelled {
            value: Value::Choice(choice),
            label: crate::args::planner_label(),
        },
    );
    ModelOutput::Calls(vec![PlannedCall {
        call: CallRequest {
            action: card.action.clone(),
            target,
            args,
            origin: Origin::Companion,
        },
        tier,
        effect: Some(card.effect),
    }])
}

/// The longest question the planner may put to the person, and its choices: the limits the tool's
/// schema states, which a model is not made to keep.
const ASK_TEXT: usize = 400;
const ASK_CHOICES: usize = 6;
const ASK_CHOICE_TEXT: usize = 80;

/// A question as the schema allows it: bounded text and at most six bounded choices, every one a
/// string. Anything else is an unreadable reply, not a question trimmed to fit.
fn question(args: &Json) -> Result<ModelOutput, PlanError> {
    let text = args
        .get("text")
        .and_then(Json::as_str)
        .filter(|t| t.chars().count() <= ASK_TEXT)
        .map(plain)
        .map(|t| without_ids(&t))
        .filter(|t| !t.trim().is_empty())
        .ok_or(PlanError::Unreadable)?;
    let choices = match args.get("choices") {
        None => Vec::new(),
        Some(Json::Array(items)) if items.len() <= ASK_CHOICES => items
            .iter()
            .map(|c| {
                c.as_str()
                    .filter(|c| c.chars().count() <= ASK_CHOICE_TEXT)
                    .map(plain)
                    .map(|t| without_ids(&t))
                    .ok_or(PlanError::Unreadable)
            })
            .collect::<Result<Vec<_>, _>>()?,
        Some(_) => return Err(PlanError::Unreadable),
    };
    Ok(ModelOutput::Ask { text, choices })
}

/// The thing a `bardo_related` is about and its kind: a handle the view holds a thing for, or a
/// thing named in full.
fn related_subject(of: &Json, view: &PlannerView) -> Option<(TargetValue, EntityKind)> {
    let object = of.as_object()?;
    if let (1, Some(n)) = (object.len(), object.get("handle")) {
        let held = Handle(n.as_u64()?);
        let kind = view.handles.iter().find_map(|c| match &c.shape {
            HandleShape::Entity(kind) if c.handle == held => Some(kind.clone()),
            _ => None,
        })?;
        return Some((TargetValue::Handles(vec![held]), kind));
    }
    let text = |name: &str| object.get(name).and_then(Json::as_str);
    let id = EntityId {
        app: AppName::parse(text("app")?).ok()?,
        kind: EntityKind::parse(text("kind")?).ok()?,
        key: EntityKey::parse(text("key")?).ok()?,
    };
    (object.len() == 3).then(|| (TargetValue::Entities(vec![id.clone()]), id.kind))
}
