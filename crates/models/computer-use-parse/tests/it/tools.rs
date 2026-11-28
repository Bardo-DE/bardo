//! Tool-call dialects: Qwen's `computer_use` and Holo 3.1's schema.

use crate::common;

use std::collections::BTreeSet;

use common::*;
use computer_use_action::{
    Button, Choice, Chord, ClickCount, ComputerUseAction, FinishOutcome, Notches, Repeat, ScrollBy,
    ScrollDir, Summary, Target, TypedText, WaitMs,
};
use computer_use_geometry::{Coord, ImageSpace, Length};
use computer_use_parse::{DropReason, ParseError};
use key_types::Modifier;
use keyboard_types::{Key, NamedKey};
use model_types::ToolDialect;
use serde_json::Value;

const QWEN: ToolDialect = ToolDialect::QwenComputerUse;
const HOLO: ToolDialect = ToolDialect::Holo31;

fn qwen(arguments: &str) -> computer_use_parse::Parsed {
    tools(QWEN, &[call("computer_use", arguments)]).unwrap()
}

fn holo(name: &str, arguments: &str) -> computer_use_parse::Parsed {
    tools(HOLO, &[call(name, arguments)]).unwrap()
}

fn fixture_calls(dir: &str, name: &str) -> Vec<model_types::ToolCall> {
    let path = format!(
        "{}/../../models/fixtures/{dir}/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    doc["calls"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| call(c["name"].as_str().unwrap(), &c["arguments"].to_string()))
        .collect()
}

#[test]
fn qwen_tool_calls_parse() {
    let p = tools(QWEN, &fixture_calls("qwen_cu", "left_click.json")).unwrap();
    assert_eq!(image_actions(&p), [left_click(at(412, 288))]);

    let p = tools(QWEN, &fixture_calls("qwen_cu", "key_then_type.json")).unwrap();
    let chord = Chord::new(BTreeSet::from([Modifier::Ctrl]), Key::Character("a".into())).unwrap();
    assert_eq!(
        image_actions(&p),
        [
            ComputerUseAction::Key {
                chord,
                repeat: Repeat::ONCE
            },
            ComputerUseAction::Type {
                text: TypedText::new("hello").unwrap()
            },
        ]
    );

    let p = tools(QWEN, &fixture_calls("qwen_cu", "scroll_and_terminate.json")).unwrap();
    assert_eq!(
        image_actions(&p),
        [
            ComputerUseAction::Scroll {
                at: at(600, 400),
                dir: ScrollDir::Down,
                by: ScrollBy::Distance(Length::<ImageSpace>::new(Coord(300))),
            },
            ComputerUseAction::Finish {
                outcome: FinishOutcome::Done,
                summary: Summary::new("").unwrap(),
                extracted: vec![],
            },
        ]
    );
    assert_eq!(p.thought, None);
}

#[test]
fn holo_tool_calls_parse() {
    let p = tools(HOLO, &fixture_calls("holo_31", "click_and_ask.json")).unwrap();
    assert_eq!(
        image_actions(&p),
        [
            left_click(at(500, 250)),
            ComputerUseAction::Ask {
                question: Summary::new("Which account?").unwrap(),
                choices: vec![
                    Choice::new("Work").unwrap(),
                    Choice::new("Personal").unwrap()
                ],
            },
        ]
    );
}

#[test]
fn qwen_click_family() {
    let click = |to, button, count| ComputerUseAction::Click {
        at: to,
        button,
        count,
        mods: BTreeSet::new(),
    };
    let mut accepted = vec![
        (
            r#"{"action":"mouse_move","coordinate":[1,2]}"#.to_owned(),
            ComputerUseAction::MoveTo { at: at(1, 2) },
        ),
        // Integral floats are numbers.
        (
            r#"{"action":"left_click","coordinate":[7.0,9]}"#.to_owned(),
            left_click(at(7, 9)),
        ),
    ];
    for (action, button, count) in [
        ("right_click", Button::Right, ClickCount::One),
        ("middle_click", Button::Middle, ClickCount::One),
        ("double_click", Button::Left, ClickCount::Two),
        ("triple_click", Button::Left, ClickCount::Three),
    ] {
        accepted.push((
            format!(r#"{{"action":"{action}","coordinate":[7,9]}}"#),
            click(at(7, 9), button, count),
        ));
    }
    for (args, want) in accepted {
        assert_eq!(image_actions(&qwen(&args)), [want], "{args}");
    }
    // Fractions are not numbers.
    let p = qwen(r#"{"action":"left_click","coordinate":[7.5,9]}"#);
    assert_eq!(drops(&p), [("left_click", DropReason::BadNumber)]);
}

#[test]
fn the_sign_of_a_scroll_is_what_the_models_are_told() {
    // The words the tool description carries...
    assert_eq!(
        computer_use_parse::SCROLL_PIXELS_RULE,
        "A scroll's `pixels` is positive to scroll up and negative to scroll down; for a horizontal scroll, positive is right and negative is left."
    );
    // ...and what both tool dialects read.
    let dir = |p: computer_use_parse::Parsed| match &image_actions(&p)[0] {
        ComputerUseAction::Scroll { dir, .. } => *dir,
        other => panic!("a scroll, got {other:?}"),
    };
    for (pixels, want) in [(30, ScrollDir::Up), (-30, ScrollDir::Down)] {
        let args = format!(r#"{{"action":"scroll","coordinate":[5,5],"pixels":{pixels}}}"#);
        assert_eq!(dir(qwen(&args)), want, "qwen {pixels}");
        assert_eq!(dir(holo("computer_use", &args)), want, "holo {pixels}");
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one scenario (or table of cases) read top to bottom; splitting it would scatter what a reader checks together"
)]
fn qwen_drag_scroll_wait_and_conclusions() {
    let by = |dir, px| ComputerUseAction::Scroll {
        at: at(5, 5),
        dir,
        by: ScrollBy::Distance(Length::new(Coord(px))),
    };
    let wait = |ms| ComputerUseAction::Wait {
        for_ms: WaitMs::new(ms).unwrap(),
    };
    let accepted = [
        (
            r#"{"action":"left_click_drag","start_coordinate":[1,2],"coordinate":[30,40]}"#,
            ComputerUseAction::Drag {
                from: at(1, 2),
                to: at(30, 40),
                button: Button::Left,
            },
        ),
        (
            r#"{"action":"scroll","coordinate":[5,5],"pixels":120}"#,
            by(ScrollDir::Up, 120),
        ),
        (
            r#"{"action":"scroll","coordinate":[5,5],"pixels":-120}"#,
            by(ScrollDir::Down, 120),
        ),
        (
            r#"{"action":"hscroll","coordinate":[5,5],"pixels":50}"#,
            by(ScrollDir::Right, 50),
        ),
        (
            r#"{"action":"hscroll","coordinate":[5,5],"pixels":-50}"#,
            by(ScrollDir::Left, 50),
        ),
        // Qwen's schema makes the coordinate optional: the scroll acts at the centre of the frame.
        (
            r#"{"action":"scroll","pixels":10}"#,
            ComputerUseAction::Scroll {
                at: Target::Centre,
                dir: ScrollDir::Up,
                by: ScrollBy::Distance(Length::new(Coord(10))),
            },
        ),
        (r#"{"action":"wait","time":1.5}"#, wait(1500)),
        (r#"{"action":"wait","time":60}"#, wait(60_000)),
    ];
    for (args, want) in accepted {
        assert_eq!(image_actions(&qwen(args)), [want], "{args}");
    }
    let dropped = [
        (
            r#"{"action":"left_click_drag","coordinate":[30,40]}"#,
            "left_click_drag",
            DropReason::MissingArgument,
        ),
        (
            r#"{"action":"scroll","coordinate":[5,5],"pixels":0}"#,
            "scroll",
            DropReason::BadNumber,
        ),
        (
            r#"{"action":"scroll","coordinate":[1],"pixels":10}"#,
            "scroll",
            DropReason::BadNumber,
        ),
        (
            r#"{"action":"scroll","x":1,"pixels":10}"#,
            "scroll",
            DropReason::MissingArgument,
        ),
        (
            r#"{"action":"wait","time":61}"#,
            "wait",
            DropReason::BadNumber,
        ),
        (
            r#"{"action":"wait","time":-1}"#,
            "wait",
            DropReason::BadNumber,
        ),
        (
            r#"{"action":"wait","time":"1e400"}"#,
            "wait",
            DropReason::BadNumber,
        ),
        (
            r#"{"action":"terminate","status":"maybe"}"#,
            "terminate",
            DropReason::BadArgument,
        ),
    ];
    for (args, verb, reason) in dropped {
        assert_eq!(drops(&qwen(args)), [(verb, reason)], "{args}");
    }

    let failed = qwen(r#"{"action":"terminate","status":"failure"}"#);
    let ComputerUseAction::Finish { outcome, .. } = &image_actions(&failed)[0] else {
        panic!()
    };
    assert_eq!(*outcome, FinishOutcome::Failed);
    let answer = qwen(r#"{"action":"answer","text":"42"}"#);
    let ComputerUseAction::Finish {
        outcome, summary, ..
    } = &image_actions(&answer)[0]
    else {
        panic!()
    };
    assert_eq!((*outcome, summary.as_str()), (FinishOutcome::Done, "42"));
}

#[test]
fn terminate_words_come_from_summary_or_text() {
    let words = |p: &computer_use_parse::Parsed| match &image_actions(p)[0] {
        ComputerUseAction::Finish {
            outcome, summary, ..
        } => (*outcome, summary.as_str().to_owned()),
        other => panic!("{other:?}"),
    };
    let done = |words: &str| (FinishOutcome::Done, words.to_owned());

    // Holo 3.1 writes its answer as `text`: that is read when there is no `summary`.
    let p = qwen(r#"{"action":"terminate","status":"success","text":"The answer is 42."}"#);
    assert_eq!(words(&p), done("The answer is 42."));
    let p = holo("finish", r#"{"status":"success","text":"Saved."}"#);
    assert_eq!(words(&p), done("Saved."));

    // `summary` alone is unchanged.
    let p = qwen(r#"{"action":"terminate","status":"success","summary":"42"}"#);
    assert_eq!(words(&p), done("42"));

    // Both: `summary` wins.
    let p = qwen(r#"{"action":"terminate","status":"success","summary":"42","text":"forty-two"}"#);
    assert_eq!(words(&p), done("42"));

    // Neither: empty, as before. Other arguments are still ignored, not read as words.
    let p = qwen(r#"{"action":"terminate","status":"success"}"#);
    assert_eq!(words(&p), done(""));
    let p = qwen(r#"{"action":"terminate","status":"success","answer":"42","note":"x"}"#);
    assert_eq!(words(&p), done(""));
    assert!(p.dropped.is_empty());

    // `answer` keeps reading `text` only, as before.
    let p = qwen(r#"{"action":"answer","text":"42"}"#);
    assert_eq!(words(&p), done("42"));
}

#[test]
fn key_spellings_in_tool_calls() {
    let chord = |args: &str| match &image_actions(&qwen(args))[0] {
        ComputerUseAction::Key { chord, .. } => chord.clone(),
        other => panic!("{other:?}"),
    };
    let want = Chord::new(BTreeSet::from([Modifier::Alt]), Key::Named(NamedKey::Tab)).unwrap();
    assert_eq!(chord(r#"{"action":"key","keys":["alt","tab"]}"#), want);
    assert_eq!(chord(r#"{"action":"key","keys":"Alt+Tab"}"#), want);
    let p = qwen(r#"{"action":"key","keys":[1,2]}"#);
    assert_eq!(drops(&p), [("key", DropReason::BadArgument)]);
    let p = qwen(r#"{"action":"key","keys":"a b"}"#);
    assert_eq!(drops(&p), [("key", DropReason::BadArgument)]);
    let p = qwen(r#"{"action":"key","keys":"ctrl+nonsense"}"#);
    assert_eq!(drops(&p), [("key", DropReason::BadArgument)]);
    let p = qwen(r#"{"action":"key"}"#);
    assert_eq!(drops(&p), [("key", DropReason::MissingArgument)]);
}

#[test]
fn unknown_verbs_and_shapes_are_dropped_not_guessed() {
    let p = qwen(r#"{"action":"open_app","name":"Files"}"#);
    assert_eq!(drops(&p), [("open_app", DropReason::UnsupportedVerb)]);
    let p = tools(QWEN, &[call("shell", r#"{"cmd":"rm -rf /"}"#)]).unwrap();
    assert_eq!(drops(&p), [("shell", DropReason::UnsupportedVerb)]);
    let p = holo("shell", r#"{"cmd":"ls"}"#);
    assert_eq!(drops(&p), [("shell", DropReason::UnsupportedVerb)]);
    // Holo's own verbs are not Qwen's.
    let p = tools(QWEN, &[call("click", r#"{"x":1,"y":2}"#)]).unwrap();
    assert_eq!(drops(&p), [("click", DropReason::UnsupportedVerb)]);
    let p = qwen(r#"{"action":"ask","question":"?"}"#);
    assert_eq!(drops(&p), [("ask", DropReason::UnsupportedVerb)]);
    let p = qwen(r#"{"coordinate":[1,2]}"#);
    assert_eq!(drops(&p), [("computer_use", DropReason::MissingArgument)]);
    for not_an_object in ["[1,2]", "7", "null", r#""click""#] {
        let p = qwen(not_an_object);
        assert_eq!(
            drops(&p),
            [("computer_use", DropReason::MissingArgument)],
            "{not_an_object}"
        );
    }
    assert_eq!(tools(QWEN, &[]), Err(ParseError::NoAction));
}

#[test]
fn holo_schema_variants() {
    let accepted = [
        ("click", r#"{"x":5,"y":6}"#, left_click(at(5, 6))),
        ("click", r#"{"coordinate":[5,6]}"#, left_click(at(5, 6))),
        // The Qwen function works under Holo too.
        (
            "computer_use",
            r#"{"action":"left_click","coordinate":[5,6]}"#,
            left_click(at(5, 6)),
        ),
        (
            "drag",
            r#"{"start_x":1,"start_y":2,"end_x":3,"end_y":4}"#,
            ComputerUseAction::Drag {
                from: at(1, 2),
                to: at(3, 4),
                button: Button::Left,
            },
        ),
        (
            "scroll",
            r#"{"x":9,"y":9,"direction":"Left"}"#,
            ComputerUseAction::Scroll {
                at: at(9, 9),
                dir: ScrollDir::Left,
                by: ScrollBy::Notches(Notches(5)),
            },
        ),
        ("observe", "{}", ComputerUseAction::Observe),
    ];
    for (name, args, want) in accepted {
        assert_eq!(image_actions(&holo(name, args)), [want], "{name} {args}");
    }
    let p = holo(
        "finish",
        r#"{"status":"infeasible","summary":"no such menu"}"#,
    );
    let ComputerUseAction::Finish {
        outcome, summary, ..
    } = &image_actions(&p)[0]
    else {
        panic!()
    };
    assert_eq!(
        (*outcome, summary.as_str()),
        (FinishOutcome::Infeasible, "no such menu")
    );
    // One coordinate alone is not a point.
    assert_eq!(
        drops(&holo("click", r#"{"x":5}"#)),
        [("click", DropReason::MissingArgument)]
    );
    let many: Vec<String> = (0..9).map(|i| format!("\"c{i}\"")).collect();
    let args = format!(r#"{{"question":"q","choices":[{}]}}"#, many.join(","));
    assert_eq!(drops(&holo("ask", &args)), [("ask", DropReason::TooLong)]);
}

#[test]
fn grid_space_checks_the_top() {
    use computer_use_geometry::{GridMax, ModelSpace};
    let space = ModelSpace::Grid(GridMax(1000));
    let calls = [
        call(
            "computer_use",
            r#"{"action":"left_click","coordinate":[1000,10]}"#,
        ),
        call(
            "computer_use",
            r#"{"action":"left_click","coordinate":[1001,10]}"#,
        ),
    ];
    let p = computer_use_parse::parse_tool_calls(
        QWEN,
        space,
        &calls,
        computer_use_parse::ParseLimits::default(),
    )
    .unwrap();
    assert_eq!(grid_actions(&p).len(), 1);
    assert_eq!(drops(&p), [("left_click", DropReason::OutOfFrame)]);
}

#[test]
fn deeply_nested_arguments_do_not_overflow() {
    let deep = format!("{}1{}", "[".repeat(5000), "]".repeat(5000));
    let args = format!(r#"{{"action":"left_click","coordinate":{deep}}}"#);
    // JsonText accepts only what serde_json can read; if it refuses, the boundary did its job.
    if let Ok(input) = model_types::JsonText::new(args) {
        let c = model_types::ToolCall {
            id: model_types::ToolCallId("x".into()),
            name: model_types::ToolName::new("computer_use").unwrap(),
            input,
        };
        let p = tools(QWEN, &[c]).unwrap();
        assert_eq!(p.dropped.len(), 1);
    }
}
