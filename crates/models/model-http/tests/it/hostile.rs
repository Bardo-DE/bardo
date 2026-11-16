//! The SSE and NDJSON framers over arbitrary bytes and arbitrary splits: no panic, a typed
//! result, the same lines however the bytes arrive, and linear time on adversarial shapes.

use model_http::{LineError, NdjsonDecoder, SseDecoder, SseError, SseEvent};

fn sse(chunks: &[&[u8]]) -> Result<Vec<SseEvent>, SseError> {
    let mut decoder = SseDecoder::new();
    let mut events = Vec::new();
    for chunk in chunks {
        events.extend(decoder.feed(chunk)?);
    }
    events.extend(decoder.finish());
    Ok(events)
}

fn ndjson(chunks: &[&[u8]]) -> Result<Vec<String>, LineError> {
    let mut decoder = NdjsonDecoder::new();
    let mut lines = Vec::new();
    for chunk in chunks {
        lines.extend(decoder.feed(chunk)?);
    }
    lines.extend(decoder.finish());
    Ok(lines)
}

#[test]
fn a_chunk_of_many_short_lines_is_linear_for_both_framers() {
    // Draining the consumed prefix once per line made these quadratic.
    let newlines = vec![b'\n'; 4 << 20];
    assert_eq!(sse(&[&newlines]), Ok(Vec::new()));
    assert_eq!(ndjson(&[&newlines]), Ok(Vec::new()));
    let crs = vec![b'\r'; 1 << 20];
    assert_eq!(sse(&[&crs]), Ok(Vec::new()));
    let events = b"data: a\n\n".repeat(200_000);
    assert_eq!(sse(&[&events]).unwrap().len(), 200_000);
    let lines = b"{}\n".repeat(400_000);
    assert_eq!(ndjson(&[&lines]).unwrap().len(), 400_000);
}

#[test]
fn a_long_line_fed_a_byte_at_a_time_is_scanned_once() {
    // Rescanning the pending line on every feed made this 4e10 byte comparisons per framer.
    let line = [b"data: ".as_slice(), &vec![b'a'; 300_000]].concat();
    let bytes: Vec<&[u8]> = line.chunks(1).collect();
    assert_eq!(sse(&bytes), Ok(Vec::new()));
    let text = vec![b'a'; 300_000];
    let bytes: Vec<&[u8]> = text.chunks(1).collect();
    assert_eq!(ndjson(&bytes), Ok(vec!["a".repeat(300_000)]));
}

#[test]
fn an_unterminated_line_over_the_cap_is_an_error_not_growth() {
    let line = vec![b'a'; (4 << 20) + 1];
    assert_eq!(ndjson(&[&line]), Err(LineError::LineTooLong));
    let line = vec![b'a'; (1 << 20) + 1];
    assert_eq!(sse(&[&line]), Err(SseError::LineTooLong));
}
