//! The bounds: input size, batch size, and the report of what was dropped.

use crate::common;

use common::*;
use computer_use_action::ToolDialect;
use computer_use_parse::{DropReason, ParseError};

#[test]
fn limits_hold_for_text() {
    let nine = format!("Action: {}", "wait()\n\n".repeat(9));
    let p = ui_tars(&nine).unwrap();
    assert_eq!(image_actions(&p).len(), 8);
    assert_eq!(drops(&p), [("wait", DropReason::OverBatchLimit)]);

    let big = format!("Action: wait() {}", " ".repeat(33 * 1024));
    assert_eq!(ui_tars(&big), Err(ParseError::TooLarge));
    // Exactly at the limit is accepted.
    let at_limit = format!(
        "Action: wait(){}",
        " ".repeat(32 * 1024 - "Action: wait()".len())
    );
    assert_eq!(at_limit.len(), 32 * 1024);
    assert!(ui_tars(&at_limit).is_ok());
}

#[test]
fn limits_hold_for_tool_calls() {
    let one = call("computer_use", r#"{"action":"wait","time":1}"#);
    let nine = vec![one.clone(); 9];
    let p = tools(ToolDialect::QwenComputerUse, &nine).unwrap();
    assert_eq!(image_actions(&p).len(), 8);
    assert_eq!(drops(&p), [("wait", DropReason::OverBatchLimit)]);

    let pad = " ".repeat(33 * 1024);
    let big = call(
        "computer_use",
        &format!(r#"{{"action":"wait","time":1}}{pad}"#),
    );
    assert_eq!(
        tools(ToolDialect::QwenComputerUse, &[big]),
        Err(ParseError::TooLarge)
    );
}

#[test]
fn the_dropped_report_is_bounded_and_clean() {
    let many = format!("Action: {}", "bogus()\n".repeat(500));
    let p = ui_tars(&many).unwrap();
    assert_eq!(p.dropped.len(), 64);
    let long = format!("Action: {}()", "v".repeat(300));
    let p = ui_tars(&long).unwrap();
    assert_eq!(p.dropped[0].verb.as_str().chars().count(), 64);
}
