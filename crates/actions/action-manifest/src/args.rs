//! The arguments of a call named in JSON, read by the types the action declares: one place for the
//! companion's tool calls and the MCP edge. The JSON is the schema `tool_schema` writes: an entity is
//! `{app, kind, key}`, a decimal `{units, scale}`, a date `{year, month, day}`, an instant or a
//! duration whole seconds, and a value the caller holds only by handle is `{ "handle": n }`. The
//! target of an action is the one extra key, `target`, whose shape follows the action's `on` (a target held
//! only by handle is `{ "handle": n }`, or a list of those for many things).
//!
//! Pure: every value comes out labelled with the label it is given, and an argument the manifest
//! does not declare is refused rather than ignored.

use crate::manifest::{ActionDecl, ParamDecl, ParamNeed, TargetKind};
use account_core::AppName;
use action_vocab::{
    Args, CharCount, CivilDate, Decimal, FileRef, Handle, ParamName, ParamType, Scale, Seconds,
    TargetValue, Value,
};
use provenance::{EntityId, EntityKey, EntityKind, Label, Labelled, UnixSeconds};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as Json};
use url::Url;

/// The key a caller names an action's target under.
pub const TARGET_KEY: &str = "target";

/// What is wrong with a call's target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum TargetError {
    /// The action acts on something and none was named.
    #[error("the action needs a target")]
    Missing,
    /// The action acts on nothing and one was named.
    #[error("the action takes no target")]
    Unexpected,
    /// A live text field cannot be named by a client.
    #[error("the target is a live text field, which a client cannot name")]
    NotNameable,
    /// The target is not of the shape (or the kind) the action declares.
    #[error("the target is malformed")]
    Malformed,
}

/// What is wrong with one argument. It carries no cause on purpose: the parse errors underneath
/// quote the text the client sent, and a refusal does not echo it into logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum ArgError {
    /// The JSON is not of the declared type.
    #[error("wrong type")]
    Type,
    /// Outside the declared range, length or options.
    #[error("out of range")]
    Range,
}

/// Why the arguments of a call were refused, before anything was sent to the router.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ArgsError {
    /// The arguments are not a JSON object.
    #[error("arguments must be an object")]
    NotAnObject,
    /// A name the action does not declare.
    #[error("unknown argument {0:?}")]
    Unknown(String),
    /// A required argument is absent.
    #[error("missing argument {0}")]
    Missing(ParamName),
    /// An argument is wrong.
    #[error("argument {param}: {why}")]
    Wrong {
        /// Which.
        param: ParamName,
        /// How.
        why: ArgError,
    },
    /// The target is wrong.
    #[error("target: {0}")]
    Target(#[from] TargetError),
}

type Read<T> = Result<T, ArgError>;

fn handle(json: &Json) -> Option<Handle> {
    let object = json.as_object()?;
    match (object.len(), object.get("handle")) {
        (1, Some(n)) => n.as_u64().map(Handle),
        _ => None,
    }
}

#[expect(
    clippy::map_err_ignore,
    reason = "`Why` carries no cause: the parse error underneath quotes the client's text"
)]
fn entity(kind: &EntityKind, json: &Json) -> Read<EntityId> {
    let object = json
        .as_object()
        .filter(|o| o.len() == 3)
        .ok_or(ArgError::Type)?;
    let text = |name: &str| {
        object
            .get(name)
            .and_then(Json::as_str)
            .ok_or(ArgError::Type)
    };
    let app = AppName::parse(text("app")?).map_err(|_| ArgError::Range)?;
    let named = EntityKind::parse(text("kind")?).map_err(|_| ArgError::Range)?;
    let key = EntityKey::parse(text("key")?).map_err(|_| ArgError::Range)?;
    if &named == kind {
        Ok(EntityId {
            app,
            kind: named,
            key,
        })
    } else {
        Err(ArgError::Range)
    }
}

fn text(max: CharCount, multiline: bool, json: &Json) -> Read<String> {
    let body = json.as_str().ok_or(ArgError::Type)?;
    let too_long = body.chars().count() > max.0 as usize;
    // One line has no control character at all; a body keeps its line breaks and tabs. Neither
    // may hold a mark that reorders the words around it or hides itself: a recipient or a
    // body drawn on a sheet must be what it reads as.
    let allowed: &[char] = if multiline { &['\n', '\r', '\t'] } else { &[] };
    let deceptive = !action_vocab::plain_text(body, allowed);
    if too_long || deceptive {
        Err(ArgError::Range)
    } else {
        Ok(body.to_owned())
    }
}

fn integer(json: &Json) -> Read<i64> {
    json.as_i64().ok_or(ArgError::Type)
}

#[expect(
    clippy::map_err_ignore,
    reason = "TryFromIntError carries nothing beyond the failure"
)]
fn small<T: TryFrom<i64>>(n: i64) -> Read<T> {
    T::try_from(n).map_err(|_| ArgError::Range)
}

fn field(object: &Map<String, Json>, name: &str) -> Read<i64> {
    object.get(name).ok_or(ArgError::Type).and_then(integer)
}

fn decimal(scale: Scale, json: &Json) -> Read<Decimal> {
    let object = json
        .as_object()
        .filter(|o| o.len() == 2)
        .ok_or(ArgError::Type)?;
    let units = field(object, "units")?;
    let given = Scale(small(field(object, "scale")?)?);
    if given == scale {
        Ok(Decimal { units, scale })
    } else {
        Err(ArgError::Range)
    }
}

fn date(json: &Json) -> Read<CivilDate> {
    let object = json
        .as_object()
        .filter(|o| o.len() == 3)
        .ok_or(ArgError::Type)?;
    let day = CivilDate {
        year: small(field(object, "year")?)?,
        month: small(field(object, "month")?)?,
        day: small(field(object, "day")?)?,
    };
    if (1..=12).contains(&day.month) && (1..=31).contains(&day.day) {
        Ok(day)
    } else {
        Err(ArgError::Range)
    }
}

#[expect(
    clippy::map_err_ignore,
    reason = "`Why` carries no cause: the parse error underneath quotes the client's text"
)]
fn url(raw: &str) -> Read<String> {
    Url::parse(raw)
        .map(|_| raw.to_owned())
        .map_err(|_| ArgError::Range)
}

/// One JSON value as a `ty`.
#[expect(
    clippy::map_err_ignore,
    reason = "`Why` carries no cause: the parse error underneath quotes the client's text"
)]
fn value(ty: &ParamType, json: &Json) -> Read<Value> {
    if let Some(held) = handle(json) {
        return match ty {
            ParamType::Text { .. }
            | ParamType::Entity(_)
            | ParamType::File
            | ParamType::Url
            | ParamType::Dynamic(_) => Ok(Value::Handle(held)),
            _ => Err(ArgError::Type),
        };
    }
    match ty {
        ParamType::Text { max, multiline } => text(*max, *multiline, json).map(Value::Text),
        ParamType::Dynamic(_) => {
            text(CharCount(crate::validate::DYNAMIC_TEXT_MAX), false, json).map(Value::Text)
        }
        ParamType::Integer { min, max } => integer(json)
            .and_then(|n| {
                (*min..=*max)
                    .contains(&n)
                    .then_some(n)
                    .ok_or(ArgError::Range)
            })
            .map(Value::Integer),
        ParamType::Decimal { scale } => decimal(*scale, json).map(Value::Decimal),
        ParamType::Date => date(json).map(Value::Date),
        ParamType::DateTime => integer(json).map(|n| Value::DateTime(UnixSeconds(n))),
        ParamType::Duration => integer(json)
            .and_then(small)
            .map(|n| Value::Duration(Seconds(n))),
        ParamType::Choice(options) => {
            let id = json.as_str().ok_or(ArgError::Type)?;
            options
                .iter()
                .find(|o| o.id.as_str() == id)
                .map(|o| Value::Choice(o.id.clone()))
                .ok_or(ArgError::Range)
        }
        ParamType::Entity(kind) => entity(kind, json).map(Value::Entity),
        ParamType::Entities(kind) => entities(kind, json),
        ParamType::File => json
            .as_str()
            .ok_or(ArgError::Type)
            .and_then(|p| FileRef::parse(p).map_err(|_| ArgError::Range))
            .map(Value::File),
        ParamType::Url => json
            .as_str()
            .ok_or(ArgError::Type)
            .and_then(url)
            .map(Value::Url),
    }
}

/// A list of things: plain entities when every one is named, a list of values when any is a
/// handle (the router resolves each with the label it was minted with).
fn entities(kind: &EntityKind, json: &Json) -> Read<Value> {
    let items = json.as_array().ok_or(ArgError::Type)?;
    if !items.iter().any(|item| handle(item).is_some()) {
        return items
            .iter()
            .map(|item| entity(kind, item))
            .collect::<Read<Vec<_>>>()
            .map(Value::Entities);
    }
    items
        .iter()
        .map(|item| match handle(item) {
            Some(held) => Ok(Value::Handle(held)),
            None => entity(kind, item).map(Value::Entity),
        })
        .collect::<Read<Vec<_>>>()
        .map(Value::List)
}

fn argument(param: &ParamDecl, json: &Json) -> Result<Value, ArgsError> {
    value(&param.ty, json).map_err(|why| ArgsError::Wrong {
        param: param.name.clone(),
        why,
    })
}

#[expect(
    clippy::map_err_ignore,
    reason = "`Why` carries no cause: the parse error underneath quotes the client's text"
)]
fn files(json: &Json) -> Read<Vec<FileRef>> {
    json.as_array()
        .ok_or(ArgError::Type)?
        .iter()
        .map(|item| {
            item.as_str()
                .ok_or(ArgError::Type)
                .and_then(|p| FileRef::parse(p).map_err(|_| ArgError::Range))
        })
        .collect()
}

/// What the action acts on, from the `target` key (absent: none).
///
/// # Errors
///
/// `TargetError` when `given` is not a target the action takes, or is missing for an action that
/// needs one.
pub fn target_from_json(
    decl: &ActionDecl,
    given: Option<&Json>,
) -> Result<TargetValue, TargetError> {
    let bad = |_: ArgError| TargetError::Malformed;
    match (&decl.on, given) {
        (TargetKind::Nothing, None) => Ok(TargetValue::Nothing),
        (TargetKind::Nothing, Some(_)) => Err(TargetError::Unexpected),
        (TargetKind::Text, _) => Err(TargetError::NotNameable),
        (_, None) => Err(TargetError::Missing),
        (TargetKind::One(_) | TargetKind::Many(_), Some(json)) if handle(json).is_some() => {
            Ok(TargetValue::Handles(handle(json).into_iter().collect()))
        }
        (TargetKind::One(kind), Some(json)) => entity(kind, json)
            .map(|e| TargetValue::Entities(vec![e]))
            .map_err(bad),
        (TargetKind::Many(kind), Some(json)) => {
            let items = json
                .as_array()
                .filter(|items| !items.is_empty())
                .ok_or(TargetError::Malformed)?;
            match items.iter().map(handle).collect::<Option<Vec<_>>>() {
                Some(held) => Ok(TargetValue::Handles(held)),
                None => items
                    .iter()
                    .map(|item| entity(kind, item))
                    .collect::<Read<Vec<_>>>()
                    .map(TargetValue::Entities)
                    .map_err(bad),
            }
        }
        (TargetKind::Files, Some(json)) => {
            let held = files(json).map_err(bad)?;
            if held.is_empty() {
                Err(TargetError::Malformed)
            } else {
                Ok(TargetValue::Files(held))
            }
        }
    }
}

/// The target and the arguments of one call, every argument labelled `label`.
///
/// # Errors
///
/// The `ArgFault` of the first argument that is missing, unknown or of the wrong type, and the
/// `TargetError` of the target.
pub fn args_from_json(
    decl: &ActionDecl,
    arguments: Option<&Map<String, Json>>,
    label: &Label,
) -> Result<(TargetValue, Args), ArgsError> {
    let empty = Map::new();
    let given = arguments.unwrap_or(&empty);
    for name in given.keys() {
        let declared = decl.params.iter().any(|p| p.name.as_str() == name);
        if !declared && name != TARGET_KEY {
            return Err(ArgsError::Unknown(name.clone()));
        }
    }
    let target = target_from_json(decl, given.get(TARGET_KEY))?;
    let mut args = Args::new();
    for param in &decl.params {
        match given.get(param.name.as_str()) {
            Some(json) => {
                args.insert(
                    param.name.clone(),
                    Labelled {
                        value: argument(param, json)?,
                        label: label.clone(),
                    },
                );
            }
            None if param.need == ParamNeed::Required => {
                return Err(ArgsError::Missing(param.name.clone()));
            }
            None => {}
        }
    }
    Ok((target, args))
}
