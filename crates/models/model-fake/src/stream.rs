//! The path the driver takes with a reply, and the invariants every such reply must keep.
//!
//! Bytes in chunks go through the SSE framer into the stream decoder. Shared by the `hostile`
//! suite of `model-openai-compat` and the model fuzz targets.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use model_http::{SseDecoder, SseError};
use model_openai_compat::{CodecError, Flavor, LeakMarker, StreamDecoder};
use model_types::{ModelName, StopReason, ToolCall, TurnEnd, TurnEvent};
use model_wire::ChatDecoder;
use serde_json::{Value, json};

/// Why a reply was refused: the framing or the decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// The SSE framer refused the bytes.
    Framing(SseError),
    /// The stream decoder refused a frame.
    Decoding(CodecError),
}

/// What came out of one reply: the events delivered, the call that leaked into text, if any, and
/// how it ended.
#[derive(Debug, Clone, PartialEq)]
pub struct Reply {
    /// The events delivered before the end.
    pub events: Vec<TurnEvent>,
    /// A tool call the decoder saw written as plain text.
    pub leak: Option<LeakMarker>,
    /// The end of the turn, or why the reply was refused.
    pub end: Result<TurnEnd, Refused>,
}

/// A decoder of the vLLM flavour, as the suites use it.
#[must_use]
pub fn decoder() -> StreamDecoder {
    StreamDecoder::new(Flavor::Vllm, ModelName("holo".into()))
}

/// Feeds `chunks` in order, then the end of the stream.
pub fn run(chunks: &[&[u8]]) -> Reply {
    let (mut framer, mut decoder) = (SseDecoder::new(), decoder());
    let mut events = Vec::new();
    let failed = |events, decoder: &StreamDecoder, why| Reply {
        events,
        leak: decoder.leaked_call(),
        end: Err(why),
    };
    for chunk in chunks {
        let found = match framer.feed(chunk) {
            Ok(found) => found,
            Err(e) => return failed(events, &decoder, Refused::Framing(e)),
        };
        for frame in found {
            match decoder.feed(&frame.data) {
                Ok(more) => events.extend(more),
                Err(e) => return failed(events, &decoder, Refused::Decoding(e)),
            }
        }
    }
    for frame in framer.finish() {
        match decoder.feed(&frame.data) {
            Ok(more) => events.extend(more),
            Err(e) => return failed(events, &decoder, Refused::Decoding(e)),
        }
    }
    let leak = decoder.leaked_call();
    let end = decoder.finish().map_err(Refused::Decoding);
    Reply { events, leak, end }
}

/// `bytes` cut at `cuts` (each taken modulo the length).
#[must_use]
pub fn run_cut(bytes: &[u8], cuts: &[usize]) -> Reply {
    let mut points: Vec<usize> = cuts.iter().map(|c| c % (bytes.len() + 1)).collect();
    points.extend([0, bytes.len()]);
    points.sort_unstable();
    let chunks: Vec<&[u8]> = points.windows(2).map(|w| &bytes[w[0]..w[1]]).collect();
    run(&chunks)
}

/// The tool calls a reply delivered whole.
#[must_use]
pub fn done(reply: &Reply) -> Vec<&ToolCall> {
    reply
        .events
        .iter()
        .filter_map(|e| match e {
            TurnEvent::ToolCallDone(call) => Some(call),
            _ => None,
        })
        .collect()
}

/// Asserts what must hold of every reply, whatever the bytes.
///
/// A delivered call was started first, has a unique id and arguments that are a whole JSON
/// object; the usage never says cached above input; a turn that ends cleanly after delivering a
/// call says it used a tool or ran out of tokens.
///
/// # Panics
///
/// When `reply` breaks one of those rules: that is the finding.
pub fn assert_sound(reply: &Reply) {
    let mut started = BTreeSet::new();
    let mut finished = BTreeSet::new();
    for event in &reply.events {
        match event {
            TurnEvent::ToolCallStarted { id, .. } => {
                assert!(started.insert(id.0.clone()), "id started twice: {id:?}");
            }
            TurnEvent::ToolCallDone(call) => {
                assert!(started.contains(&call.id.0), "done without start");
                assert!(finished.insert(call.id.0.clone()), "done twice");
                let parsed: Value =
                    serde_json::from_str(call.input.as_str()).expect("delivered arguments parse");
                assert!(parsed.is_object(), "arguments are an object");
            }
            TurnEvent::Usage(u) => assert!(u.cached <= u.input),
            _ => {}
        }
    }
    if let Ok(end) = &reply.end {
        assert!(end.usage.cached <= end.usage.input);
        // A call that arrived whole before a `length` cut (or an unrecognized stop) stands (its arguments are a complete
        // object); the turn then says `MaxTokens`.
        if !finished.is_empty() {
            assert!(matches!(
                end.stop,
                StopReason::ToolUse | StopReason::MaxTokens | StopReason::Unrecognized
            ));
        }
        // A clean end leaves no call half-delivered, except one the token budget cut off (a
        // started call is then dropped, never completed).
        if !matches!(end.stop, StopReason::MaxTokens | StopReason::Unrecognized) {
            assert_eq!(
                started.len(),
                finished.len(),
                "a started call was never done"
            );
        }
    }
}

/// `frames` as a server-sent-event body, closed by `[DONE]`.
#[must_use]
pub fn sse(frames: &[String]) -> Vec<u8> {
    let mut body = String::new();
    for frame in frames {
        write!(body, "data: {frame}\n\n").expect("a String takes writes");
    }
    body.push_str("data: [DONE]\n\n");
    body.into_bytes()
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "a test helper whose callers build the JSON inline with json!(); taking it by reference would put & in front of every call"
)]
/// One chat-completion chunk with `delta` and the finish reason.
#[must_use]
pub fn chunk(delta: Value, finish: Option<&str>) -> String {
    json!({"id":"c","object":"chat.completion.chunk","model":"holo",
           "choices":[{"index":0,"delta":delta,"finish_reason":finish}]})
    .to_string()
}

/// One tool-call fragment of a delta.
#[must_use]
pub fn call(index: u32, id: Option<&str>, name: Option<&str>, args: Option<&str>) -> Value {
    let mut c = json!({"index": index, "type": "function", "function": {}});
    if let Some(id) = id {
        c["id"] = json!(id);
    }
    if let Some(name) = name {
        c["function"]["name"] = json!(name);
    }
    if let Some(args) = args {
        c["function"]["arguments"] = json!(args);
    }
    c
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "a test helper whose callers build the JSON inline with json!(); taking it by reference would put & in front of every call"
)]
/// A delta that carries `list` as its tool calls.
#[must_use]
pub fn calls(list: Vec<Value>) -> Value {
    json!({ "tool_calls": list })
}

/// A valid reply with two calls, text with multi-byte characters, a thought, and usage.
#[must_use]
pub fn scenario() -> Vec<String> {
    vec![
        chunk(
            json!({"role":"assistant","reasoning_content":"思考 "}),
            None,
        ),
        chunk(json!({"content":"héllo 世界 "}), None),
        chunk(
            calls(vec![call(0, Some("a"), Some("f"), Some("{\"x\""))]),
            None,
        ),
        chunk(calls(vec![call(1, Some("b"), Some("g"), Some("{}"))]), None),
        chunk(calls(vec![call(0, None, None, Some(":\"ü\"}"))]), None),
        chunk(json!({}), Some("tool_calls")),
        json!({"id":"c","choices":[],"usage":{"prompt_tokens":9,"completion_tokens":4}})
            .to_string(),
    ]
}
