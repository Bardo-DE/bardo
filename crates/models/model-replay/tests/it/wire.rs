use base_ids::Tokens;
use model_http::{BodyKind, EventName, HttpStatus, RouteRoot, UrlPath, Verb, WaitSeconds};
use model_replay::{
    BuildLabel, CassetteError, CassetteHeader, CassetteVersion, ContextStamp, EngineLabel,
    EngineStamp, HeadPrint, RecordedAt, WireBody, WireCassette, WireEnd, WireExchange, WireFrame,
    WireReply, WireRequest,
};
use model_types::{JsonText, ModelName};

fn header() -> CassetteHeader {
    CassetteHeader {
        vocab: CassetteVersion::CURRENT,
        engine: EngineStamp {
            kind: EngineLabel("llama_server".into()),
            build: BuildLabel("b10964-b29c606e2".into()),
        },
        model: ModelName("holo".into()),
        recorded: RecordedAt(1_790_000_000),
        context: ContextStamp {
            loaded: Tokens(8192),
            trained: Tokens(32768),
        },
        speech: None,
    }
}

fn streaming() -> WireExchange {
    WireExchange {
        request: WireRequest {
            verb: Verb::PostJson,
            root: RouteRoot::Base,
            path: UrlPath("/chat/completions".into()),
            body: Some(JsonText::new(r#"{"model":"holo","stream":true}"#).unwrap()),
        },
        reply: WireReply {
            head: HeadPrint {
                status: HttpStatus(200),
                body: BodyKind::EventStream,
                retry_after: None,
            },
            body: WireBody::Frames(vec![
                WireFrame {
                    event: None,
                    data: r#"{"choices":[{"delta":{"content":"hi"}}]}"#.into(),
                },
                WireFrame {
                    event: Some(EventName("ping".into())),
                    data: String::new(),
                },
                WireFrame {
                    event: None,
                    data: "[DONE]".into(),
                },
            ]),
            end: WireEnd::Complete,
        },
    }
}

fn overloaded() -> WireExchange {
    WireExchange {
        request: WireRequest {
            verb: Verb::Get,
            root: RouteRoot::Server,
            path: UrlPath("/props".into()),
            body: None,
        },
        reply: WireReply {
            head: HeadPrint {
                status: HttpStatus(503),
                body: BodyKind::Json,
                retry_after: Some(WaitSeconds(2)),
            },
            body: WireBody::Whole(r#"{"error":{"message":"loading"}}"#.into()),
            end: WireEnd::Cut,
        },
    }
}

fn cassette() -> WireCassette {
    WireCassette {
        header: header(),
        exchanges: vec![streaming(), overloaded()],
    }
}

#[test]
fn a_wire_cassette_round_trips_as_jsonl() {
    let text = cassette().to_jsonl();
    assert_eq!(
        text.lines().count(),
        3,
        "a header and one line per exchange"
    );
    assert_eq!(WireCassette::from_jsonl(&text).unwrap(), cassette());
}

#[test]
fn bad_wire_files_name_the_line() {
    assert!(matches!(
        WireCassette::from_jsonl(""),
        Err(CassetteError::Empty)
    ));
    let mut text = cassette().to_jsonl();
    text.push_str("{nope}\n");
    assert!(matches!(
        WireCassette::from_jsonl(&text),
        Err(CassetteError::BadLine { line: 4, .. })
    ));
}
