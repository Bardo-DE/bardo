//! SSE and NDJSON through the wire cassette: what a recording makes of the raw bytes a server
//! writes (CRLF, comments, a split code point, a stream cut in the middle).

use std::fmt::Write as _;
use std::ops::ControlFlow;
use std::sync::Mutex;

use model_http::{
    BodyKind, BodySink, EventName, Exchange, Framing, HttpError, HttpStatus, JsonBody,
    ResponseHead, RouteRoot, Transport, UrlPath, Verb,
};
use model_provider::{ModelName, Tokens};
use model_replay::{
    BuildLabel, CassetteHeader, CassetteVersion, ContextStamp, EngineLabel, EngineStamp,
    RecordedAt, RecordingTransport, SinkError, WireBody, WireCassette, WireEnd, WireExchange,
    WireFrame, WireSink,
};
use proptest::prelude::*;

fn block_on<T>(future: impl Future<Output = T>) -> T {
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};
    match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("these transports never wait"),
    }
}

/// A server that writes `chunks` after a head and then ends with `result`.
struct Server {
    kind: BodyKind,
    chunks: Vec<Vec<u8>>,
    result: Result<HttpStatus, HttpError>,
}

impl Transport for Server {
    fn exchange<K: BodySink>(
        &self,
        _ex: &Exchange,
        sink: &mut K,
    ) -> impl Future<Output = Result<HttpStatus, HttpError>> + Send {
        let head = ResponseHead {
            status: HttpStatus(200),
            body: self.kind,
            retry_after: None,
            request_id: None,
            location: None,
            content_range: None,
        };
        let mut flow = sink.head(&head);
        for chunk in &self.chunks {
            if flow == ControlFlow::Break(()) {
                break;
            }
            flow = sink.chunk(chunk);
        }
        std::future::ready(self.result)
    }
}

#[derive(Default)]
struct Written(Mutex<Vec<WireExchange>>);

impl WireSink for &Written {
    fn write(&self, exchange: &WireExchange) -> Result<(), SinkError> {
        self.0.lock().unwrap().push(exchange.clone());
        Ok(())
    }
}

struct Nothing;

impl BodySink for Nothing {
    fn head(&mut self, _head: &ResponseHead) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn chunk(&mut self, _bytes: &[u8]) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
}

fn exchange(framing: Framing) -> Exchange {
    Exchange {
        verb: Verb::PostJson,
        root: RouteRoot::Base,
        path: UrlPath("/chat/completions".into()),
        body: Some(JsonBody(r#"{"stream":true}"#.into())),
        framing,
    }
}

/// What recording `chunks` as `kind` under `framing` writes to the cassette.
fn record(
    kind: BodyKind,
    framing: Framing,
    chunks: &[&[u8]],
    result: Result<HttpStatus, HttpError>,
) -> WireExchange {
    let written = Written::default();
    let server = Server {
        kind,
        chunks: chunks.iter().map(|c| c.to_vec()).collect(),
        result,
    };
    let recording = RecordingTransport::new(server, &written);
    let _ = block_on(recording.exchange(&exchange(framing), &mut Nothing));
    let mut all = written.0.lock().unwrap().clone();
    assert_eq!(all.len(), 1);
    all.remove(0)
}

fn frames(exchange: &WireExchange) -> Vec<(Option<String>, String)> {
    match &exchange.reply.body {
        WireBody::Frames(frames) => frames
            .iter()
            .map(|f| (f.event.as_ref().map(|e| e.0.clone()), f.data.clone()))
            .collect(),
        WireBody::Whole(text) => panic!("recorded whole: {text:?}"),
    }
}

fn pair(event: Option<&str>, data: &str) -> (Option<String>, String) {
    (event.map(str::to_owned), data.to_owned())
}

#[test]
fn an_sse_body_is_recorded_as_events_whatever_its_line_endings() {
    let cases: &[(&str, &[u8])] = &[
        ("lf", b"data: a\n\ndata: b\n\n"),
        ("crlf", b"data: a\r\n\r\ndata: b\r\n\r\n"),
        ("cr", b"data: a\r\rdata: b\r\r"),
        ("mixed", b"data: a\r\n\ndata: b\n\r\n"),
        ("no space after the colon", b"data:a\n\ndata:b\n\n"),
        (
            "comment keep-alives",
            b": keep-alive\n\ndata: a\n: ping\n\n:\ndata: b\n\n",
        ),
        ("a bom", b"\xEF\xBB\xBFdata: a\n\ndata: b\n\n"),
    ];
    for (name, bytes) in cases {
        let got = record(
            BodyKind::EventStream,
            Framing::Sse,
            &[bytes],
            Ok(HttpStatus(200)),
        );
        assert_eq!(
            frames(&got),
            vec![pair(None, "a"), pair(None, "b")],
            "{name}"
        );
        assert_eq!(got.reply.end, WireEnd::Complete, "{name}");
    }
}

#[test]
fn event_names_and_multi_line_data_survive() {
    let got = record(
        BodyKind::EventStream,
        Framing::Sse,
        &[b"event: message_start\ndata: {\"a\":\ndata: 1}\n\nevent: ping\ndata: x\n\ndata: y\n\n"],
        Ok(HttpStatus(200)),
    );
    assert_eq!(
        frames(&got),
        vec![
            pair(Some("message_start"), "{\"a\":\n1}"),
            pair(Some("ping"), "x"),
            pair(None, "y"),
        ]
    );
}

#[test]
fn the_recording_does_not_depend_on_how_the_server_chunked_its_bytes() {
    let body =
        "event: e\r\ndata: caf\u{e9} \u{1F600}\r\n\r\n: c\r\ndata: [DONE]\r\n\r\n".as_bytes();
    let whole = record(
        BodyKind::EventStream,
        Framing::Sse,
        &[body],
        Ok(HttpStatus(200)),
    );
    assert_eq!(
        frames(&whole),
        vec![pair(Some("e"), "caf\u{e9} \u{1F600}"), pair(None, "[DONE]")]
    );
    for step in 1..body.len() {
        let chunks: Vec<&[u8]> = body.chunks(step).collect();
        let got = record(
            BodyKind::EventStream,
            Framing::Sse,
            &chunks,
            Ok(HttpStatus(200)),
        );
        assert_eq!(
            got, whole,
            "step {step}: a code point or a CRLF split across chunks"
        );
    }
}

#[test]
fn a_stream_cut_mid_event_records_only_the_events_that_finished() {
    let got = record(
        BodyKind::EventStream,
        Framing::Sse,
        &[b"data: a\n\ndata: half"],
        Err(HttpError::Broken),
    );
    assert_eq!(frames(&got), vec![pair(None, "a")]);
    assert_eq!(got.reply.end, WireEnd::Reset);
}

#[test]
fn an_sse_body_that_is_not_utf8_is_recorded_whole_not_lost() {
    let got = record(
        BodyKind::EventStream,
        Framing::Sse,
        &[b"data: \xFF\xFE\n\n"],
        Ok(HttpStatus(200)),
    );
    assert!(matches!(got.reply.body, WireBody::Whole(_)));
}

#[test]
fn a_framing_that_does_not_match_the_head_is_recorded_whole() {
    let got = record(
        BodyKind::Json,
        Framing::Sse,
        &[br#"{"error":"busy"}"#],
        Ok(HttpStatus(200)),
    );
    assert_eq!(
        got.reply.body,
        WireBody::Whole(r#"{"error":"busy"}"#.into())
    );
}

#[test]
fn ndjson_lines_are_frames_and_blank_lines_are_not() {
    let cases: &[(&str, &[&[u8]])] = &[
        ("lf", &[b"{\"a\":1}\n{\"b\":2}\n"]),
        ("crlf", &[b"{\"a\":1}\r\n{\"b\":2}\r\n"]),
        ("blank lines", &[b"\n{\"a\":1}\n\n\r\n{\"b\":2}\n\n"]),
        ("split lines", &[b"{\"a\"", b":1}\n{\"b", b"\":2}\n"]),
        ("an unterminated last line", &[b"{\"a\":1}\n{\"b\":2}"]),
    ];
    for (name, chunks) in cases {
        let got = record(
            BodyKind::NdJson,
            Framing::Ndjson,
            chunks,
            Ok(HttpStatus(200)),
        );
        assert_eq!(
            frames(&got),
            vec![pair(None, r#"{"a":1}"#), pair(None, r#"{"b":2}"#)],
            "{name}"
        );
    }
}

fn cassette(exchange: WireExchange) -> WireCassette {
    WireCassette {
        header: CassetteHeader {
            vocab: CassetteVersion::CURRENT,
            engine: EngineStamp {
                kind: EngineLabel("vllm".into()),
                build: BuildLabel("v0".into()),
            },
            model: ModelName("m".into()),
            recorded: RecordedAt(1),
            context: ContextStamp {
                loaded: Tokens(1),
                trained: Tokens(1),
            },
            speech: None,
        },
        exchanges: vec![exchange],
    }
}

#[test]
fn the_cassette_file_of_a_streamed_exchange_reads_back_equal() {
    let recorded = record(
        BodyKind::EventStream,
        Framing::Sse,
        &[b"event: e\ndata: {\"a\":1}\n\ndata: [DONE]\n\n"],
        Ok(HttpStatus(200)),
    );
    let file = cassette(recorded).to_jsonl();
    assert_eq!(file.lines().count(), 2, "a header and one exchange");
    assert_eq!(WireCassette::from_jsonl(&file).unwrap().to_jsonl(), file);
}

fn data_line() -> impl Strategy<Value = String> {
    // Anything but a line break: the SSE grammar has no escape for one inside a data line.
    "[^\r\n\u{0}]{0,24}"
}

proptest! {
    #[test]
    fn any_events_recorded_from_any_chunking_come_back_as_the_same_frames(
        events in proptest::collection::vec(
            (proptest::option::of("[a-z_]{1,8}"), proptest::collection::vec(data_line(), 1..4)),
            1..6,
        ),
        endings in proptest::sample::select(vec!["\n", "\r\n", "\r"]),
        cuts in proptest::collection::vec(any::<usize>(), 0..8),
    ) {
        let mut raw = String::new();
        for (name, lines) in &events {
            if let Some(name) = name {
                write!(raw, "event: {name}{endings}").expect("writing to a String cannot fail");
            }
            for line in lines {
                write!(raw, "data: {line}{endings}").expect("writing to a String cannot fail");
            }
            raw.push_str(endings);
        }
        let bytes = raw.as_bytes();
        let mut points: Vec<usize> = cuts.iter().map(|c| c % (bytes.len() + 1)).collect();
        points.sort_unstable();
        points.dedup();
        let mut chunks: Vec<&[u8]> = Vec::new();
        let mut from = 0;
        for at in points {
            chunks.push(&bytes[from..at]);
            from = at;
        }
        chunks.push(&bytes[from..]);
        let got = record(BodyKind::EventStream, Framing::Sse, &chunks, Ok(HttpStatus(200)));
        let want: Vec<WireFrame> = events
            .iter()
            .map(|(name, lines)| WireFrame {
                event: name.clone().map(EventName),
                data: lines.join("\n"),
            })
            .collect();
        prop_assert_eq!(got.reply.body, WireBody::Frames(want));
    }
}
