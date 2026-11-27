//! Named mutations of valid replies, each with the outcome the decoder owes.

use model_http::SseError;
use model_openai_compat::{CodecError, LeakMarker};
use model_types::{StopReason, TurnEvent};
use serde_json::json;

use model_wire::ChatDecoder;

use model_fake::stream::{
    Refused, Reply, assert_sound, call, calls, chunk, done, run, scenario, sse,
};

fn go(frames: &[String]) -> Reply {
    let reply = run(&[&sse(frames)]);
    assert_sound(&reply);
    reply
}

fn stop(reply: &Reply) -> Option<StopReason> {
    reply.end.as_ref().ok().map(|e| e.stop)
}

#[test]
fn the_valid_scenario_is_the_baseline() {
    let reply = go(&scenario());
    assert_eq!(done(&reply).len(), 2);
    assert_eq!(stop(&reply), Some(StopReason::ToolUse));
}

#[test]
fn a_stream_cut_at_every_byte_ends_cleanly_only_after_its_finish_reason() {
    let frames = scenario();
    let finish_at: usize = frames[..6]
        .iter()
        .map(|f| format!("data: {f}\n\n").len())
        .sum();
    let full = sse(&frames);
    for cut in 0..full.len() {
        let reply = run(&[&full[..cut]]);
        assert_sound(&reply);
        if reply.end.is_ok() {
            assert!(
                cut >= finish_at,
                "cut {cut} ended cleanly before the finish reason"
            );
        }
    }
}

#[test]
fn an_index_beyond_u32_a_duplicate_id_or_a_bad_name_is_unreadable() {
    let big = chunk(
        calls(vec![
            json!({"index": 1u64 << 40, "id":"a","function":{"name":"f"}}),
        ]),
        None,
    );
    let dup = [
        chunk(calls(vec![call(0, Some("a"), Some("f"), None)]), None),
        chunk(calls(vec![call(1, Some("a"), Some("f"), None)]), None),
    ];
    let name = chunk(
        calls(vec![call(0, Some("a"), Some("bad name!"), None)]),
        None,
    );
    for frames in [vec![big], dup.to_vec(), vec![name]] {
        assert_eq!(
            go(&frames).end,
            Err(Refused::Decoding(CodecError::Unreadable))
        );
    }
}

#[test]
fn more_calls_than_the_limit_is_unreadable() {
    let frames: Vec<String> = (0..300)
        .map(|i| {
            chunk(
                calls(vec![call(
                    i,
                    Some(&format!("id{i}")),
                    Some("f"),
                    Some("{}"),
                )]),
                None,
            )
        })
        .collect();
    let reply = go(&frames);
    assert_eq!(reply.end, Err(Refused::Decoding(CodecError::Unreadable)));
    assert!(done(&reply).is_empty());
}

#[test]
fn content_and_calls_after_the_finish_reason_are_unreadable() {
    for late in [
        chunk(json!({"content":"more"}), None),
        chunk(calls(vec![call(1, Some("z"), Some("f"), Some("{}"))]), None),
    ] {
        let reply = go(&[chunk(json!({"content":"hi"}), Some("stop")), late]);
        assert_eq!(reply.end, Err(Refused::Decoding(CodecError::Unreadable)));
    }
    // A second finish reason does not replace the first; empty trailing deltas are tolerated.
    let reply = go(&[
        chunk(json!({"content":"hi"}), Some("stop")),
        chunk(json!({}), Some("length")),
    ]);
    assert_eq!(stop(&reply), Some(StopReason::EndTurn));
}

#[test]
fn invalid_utf8_in_a_line_is_a_framing_error_and_split_characters_are_not() {
    let reply = run(&[b"data: {\"a\":\"\xff\xfe\"}\n\n"]);
    assert!(matches!(
        reply.end,
        Err(Refused::Framing(SseError::NotUtf8(_)))
    ));
    let body = sse(&scenario());
    let at = body.iter().position(|b| *b == 0xE4).unwrap();
    let split = run(&[&body[..=at], &body[at + 1..at + 2], &body[at + 2..]]);
    assert_eq!(split, run(&[&body]));
}

#[test]
fn a_call_left_in_the_content_is_text_never_a_call_and_is_flagged() {
    let json_call = "<tool_call>{\"name\":\"f\",\"arguments\":{\"x\":1}}</tool_call>";
    let xml_call = "<function=f><parameter=x>1</parameter></function>";
    for (text, marker) in [
        (json_call, LeakMarker::ToolCallTag),
        (xml_call, LeakMarker::QwenFunctionTag),
        (
            "<tool_call>\n<function=f></function>\n</tool_call>",
            LeakMarker::ToolCallTag,
        ),
        (
            "[TOOL_CALLS][{\"name\":\"f\"}]",
            LeakMarker::MistralToolCalls,
        ),
    ] {
        let (head, tail) = text.split_at(text.len() / 2);
        let reply = go(&[
            chunk(json!({"content": head}), None),
            chunk(json!({"content": tail}), Some("stop")),
        ]);
        assert!(done(&reply).is_empty(), "{text}");
        assert_eq!(stop(&reply), Some(StopReason::EndTurn), "{text}");
        assert_eq!(reply.leak, Some(marker), "{text}");
        let said: String = reply
            .events
            .iter()
            .filter_map(|e| match e {
                TurnEvent::TextDelta(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(said, text);
    }
}

#[test]
fn prose_is_not_flagged_and_a_real_call_beside_a_mention_is_not_a_leak() {
    let reply = go(&[chunk(json!({"content":"all done"}), Some("stop"))]);
    assert_eq!(reply.leak, None);
    let reply = go(&[
        chunk(json!({"content":"I will use <tool_call> syntax"}), None),
        chunk(
            calls(vec![call(0, Some("a"), Some("f"), Some("{}"))]),
            Some("tool_calls"),
        ),
    ]);
    assert_eq!(done(&reply).len(), 1);
    assert_eq!(reply.leak, None);
}

#[test]
fn a_reply_over_the_stream_limit_is_unreadable() {
    // Frames are read straight from the decoder: one SSE line is capped far below this.
    let big = chunk(json!({"content": "a".repeat(4 << 20)}), None);
    let mut d = model_fake::stream::decoder();
    let results: Vec<_> = (0..17).map(|_| d.feed(&big)).collect();
    assert!(results[..15].iter().all(Result::is_ok));
    assert_eq!(results[16], Err(CodecError::Unreadable));
}

#[test]
fn an_sse_line_or_event_over_its_limit_is_a_framing_error() {
    let long = format!("data: {}\n", "a".repeat((1 << 20) + 1));
    assert_eq!(
        run(&[long.as_bytes()]).end,
        Err(Refused::Framing(SseError::LineTooLong))
    );
    // An unterminated line cannot grow without end either.
    let bytes = vec![b'a'; (1 << 20) + 2];
    let chunks: Vec<&[u8]> = bytes.chunks(64 << 10).collect();
    assert_eq!(
        run(&chunks).end,
        Err(Refused::Framing(SseError::LineTooLong))
    );
    let line = format!("data: {}\n", "a".repeat(1 << 19));
    let event = line.repeat(17);
    assert_eq!(
        run(&[event.as_bytes()]).end,
        Err(Refused::Framing(SseError::LineTooLong))
    );
}

#[test]
fn many_short_lines_and_one_byte_chunks_cost_linear_time() {
    // Each of these was quadratic: the consumed prefix was drained per line, and an unfinished
    // line was rescanned per chunk.
    let newlines = vec![b'\n'; 4 << 20];
    run(&[&newlines]).end.unwrap_err();
    let crs = vec![b'\r'; 1 << 20];
    run(&[&crs]).end.unwrap_err();
    let line = [b"data: ".as_slice(), &vec![b'a'; 400_000]].concat();
    let bytes: Vec<&[u8]> = line.chunks(1).collect();
    run(&bytes).end.unwrap_err();
}

#[test]
fn an_embedding_outside_f32_range_is_unreadable_not_infinite() {
    use model_openai_compat::{Flavor, OpenAiCodec};
    use model_types::ModelName;
    use model_wire::EmbedCodec;
    let codec = OpenAiCodec::new(Flavor::Vllm);
    let decode = |body: &str| codec.decode_embed(ModelName("m".into()), body.as_bytes());
    assert_eq!(
        decode(r#"{"data":[{"embedding":[0.5,1e300]}]}"#),
        Err(CodecError::Unreadable)
    );
    decode(r#"{"data":[{"embedding":[0.5,-1e30]}]}"#).unwrap();
}
