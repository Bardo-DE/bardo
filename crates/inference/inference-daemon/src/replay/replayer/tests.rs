use super::*;
use crate::replay::cassette::{TEST_HEADER, Uses};
use base_testkit::Scratch;
use serde_json::json;

fn scratch(dir: &Scratch, name: &str, text: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, text).expect("write");
    path
}

#[test]
fn a_missing_or_malformed_file_is_a_typed_error() {
    let missing = Replayer::load(Path::new("/nonexistent/cassette.jsonl")).expect_err("missing");
    assert!(matches!(
        &missing,
        ReplayError::Unreadable { path, source }
            if path == "/nonexistent/cassette.jsonl"
                && source.kind() == std::io::ErrorKind::NotFound
    ));
    assert_eq!(missing.slug(), "replay_unreadable");
    let dir = Scratch::new("replayer-bad");
    let path = scratch(&dir, "bad", "not json");
    let bad = Replayer::load(&path).expect_err("malformed");
    assert!(matches!(
        &bad,
        ReplayError::Malformed { path: shown, why }
            if *shown == path.display().to_string()
                && matches!(**why, CassetteError::BadLine { line: 1, .. })
    ));
    assert_eq!(bad.slug(), "replay_malformed");
}

#[test]
fn requests_match_in_order_and_a_miss_is_a_typed_error() {
    let text = format!(
        "{TEST_HEADER}\n{}\n{}",
        r#"{"when":{"tools":"present"},"reply":{"kind":"text","v":"first"}}"#,
        r#"{"when":{"tools":"present"},"reply":{"kind":"fail","v":503}}"#
    );
    let dir = Scratch::new("replayer-ok");
    let replayer = Replayer::load(&scratch(&dir, "ok", &text)).expect("loads");
    let planner = json!({"messages": [], "tools": [{"type": "function"}]});
    assert_eq!(
        replayer.answer(&planner).expect("first entry"),
        Reply::Text("first".into())
    );
    assert_eq!(
        replayer.answer(&planner).expect("second entry"),
        Reply::Fail(503)
    );
    let miss = replayer.answer(&planner).expect_err("spent");
    assert!(matches!(
        miss,
        ReplayError::NoEntry {
            entries: 2,
            answered: 2
        }
    ));
    assert_eq!(miss.slug(), "replay_miss");
    let plain = json!({"messages": [{"role": "user", "content": "hi"}]});
    assert!(
        replayer.answer(&plain).is_err(),
        "no entry for a request without tools"
    );
    assert_eq!(Uses::default(), Uses::Once);
}
