//! The pure readings of a response head: what a `Content-Type` says and what `Retry-After` means.

use http_core::http::{HeaderMap, HeaderValue, StatusCode};
use model_http::{BodyKind, ResponseHead, WaitSeconds};

fn retry_after_of(value: &str) -> Option<WaitSeconds> {
    let mut headers = HeaderMap::new();
    headers.insert("retry-after", HeaderValue::from_str(value).expect("a header value"));
    ResponseHead::of(StatusCode::TOO_MANY_REQUESTS, &headers).retry_after
}

#[test]
fn a_content_type_names_the_kind_of_body() {
    let cases = [
        ("text/event-stream", BodyKind::EventStream),
        ("text/event-stream; charset=utf-8", BodyKind::EventStream),
        ("TEXT/Event-Stream", BodyKind::EventStream),
        ("application/json", BodyKind::Json),
        ("application/json; charset=utf-8", BodyKind::Json),
        ("application/problem+json", BodyKind::Json),
        ("application/x-ndjson", BodyKind::NdJson),
        ("application/jsonl", BodyKind::NdJson),
        ("text/html", BodyKind::Html),
        ("text/html; charset=UTF-8", BodyKind::Html),
        ("application/xhtml+xml", BodyKind::Html),
        ("text/plain", BodyKind::Other),
        ("application/octet-stream", BodyKind::Other),
        ("", BodyKind::Other),
        ("garbage", BodyKind::Other),
    ];
    for (content_type, want) in cases {
        assert_eq!(BodyKind::of(content_type), want, "{content_type:?}");
    }
}

#[test]
fn retry_after_is_its_seconds_and_a_date_with_no_date_header_is_none() {
    let cases = [
        ("7", Some(7)),
        (" 30 ", Some(30)),
        ("0", Some(0)),
        ("4294967295", Some(u32::MAX)),
        ("4294967296", None),
        ("-1", None),
        ("1.5", None),
        ("", None),
        ("Wed, 21 Oct 2026 07:28:00 GMT", None),
        ("5s", None),
    ];
    for (value, want) in cases {
        assert_eq!(retry_after_of(value), want.map(WaitSeconds), "{value:?}");
    }
}

#[test]
fn any_text_is_a_kind_and_never_a_panic() {
    let mut runner = proptest::test_runner::TestRunner::default();
    runner
        .run(&".{0,60}", |text| {
            let _ = BodyKind::of(&text);
            let _ = HeaderValue::from_str(&text).map(|v| {
                let mut headers = HeaderMap::new();
                headers.insert("retry-after", v);
                ResponseHead::of(StatusCode::OK, &headers)
            });
            Ok(())
        })
        .unwrap();
}
