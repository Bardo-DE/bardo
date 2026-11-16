use super::*;
use serde_json::json;

fn file(lines: &[&str]) -> String {
    std::iter::once(TEST_HEADER)
        .chain(lines.iter().copied())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_cassette_reads_or_says_why_not() {
    let entry = r#"{"when":{"tools":"present"},"reply":{"kind":"text","v":"hi"}}"#;
    // An error is named by its kind and the line it points at.
    let cases: Vec<(&str, String, Result<usize, &str>)> = vec![
        ("one entry", file(&[entry]), Ok(1)),
        ("a blank line is skipped", file(&["", entry, ""]), Ok(1)),
        ("header only", file(&[]), Ok(0)),
        ("empty", String::new(), Err("empty")),
        ("bad header", "{}".to_owned(), Err("bad line 1")),
        ("bad entry", file(&["{\"when\":1}"]), Err("bad line 2")),
        (
            "unknown version",
            TEST_HEADER.replace("\"vocab\":1", "\"vocab\":9"),
            Err("version 9"),
        ),
    ];
    for (name, text, want) in cases {
        let got = Cassette::parse(&text)
            .map(|c| c.entries.len())
            .map_err(|error| match error {
                CassetteError::Empty => "empty".to_owned(),
                CassetteError::BadLine { line, .. } => format!("bad line {line}"),
                CassetteError::BadSequence { line, .. } => format!("bad sequence {line}"),
                CassetteError::Version { found, want } => {
                    assert_eq!(want, CassetteVersion::CURRENT, "{name}");
                    format!("version {}", found.0)
                }
            });
        assert_eq!(got, want.map_err(str::to_owned), "{name}");
    }
}

fn entry(tools: Tools, contains: &[&str], uses: Uses) -> Entry {
    Entry {
        when: When {
            tools,
            contains: contains.iter().map(|s| (*s).to_owned()).collect(),
            lacks: vec![],
        },
        reply: Reply::Text("x".into()),
        uses,
    }
}

#[test]
fn an_entry_admits_by_tools_and_words() {
    let planner = Seen::of(&json!({
        "messages": [{"role": "user", "content": [{"type": "text", "text": "forward it"}]}],
        "tools": [{"type": "function"}]
    }));
    let writer = Seen::of(&json!({
        "messages": [{"role": "system", "content": "You write a task policy"}]
    }));
    assert!(planner.tools && !writer.tools);
    let cases = [
        (Tools::Present, vec![], true, false),
        (Tools::Absent, vec![], false, true),
        (Tools::Any, vec!["task policy"], false, true),
        (Tools::Present, vec!["forward"], true, false),
        (Tools::Present, vec!["forward", "absent"], false, false),
    ];
    for (tools, words, on_planner, on_writer) in cases {
        let when = entry(tools, &words, Uses::Once).when;
        assert_eq!(when.admits(&planner), on_planner, "{tools:?} {words:?}");
        assert_eq!(when.admits(&writer), on_writer, "{tools:?} {words:?}");
    }
    let lacking = When {
        lacks: vec!["forward".into()],
        ..When::default()
    };
    assert!(!lacking.admits(&planner) && lacking.admits(&writer));
}

#[test]
fn pick_takes_entries_in_order_and_spends_the_once_ones() {
    let cassette = Cassette::parse(&file(&[])).expect("header");
    let cassette = Cassette {
        entries: vec![
            entry(Tools::Present, &[], Uses::Once),
            entry(Tools::Present, &[], Uses::Once),
            entry(Tools::Absent, &[], Uses::Always),
        ],
        ..cassette
    };
    let tools = Seen {
        tools: true,
        text: String::new(),
    };
    let plain = Seen {
        tools: false,
        text: String::new(),
    };
    assert_eq!(cassette.pick(&tools, &[]), Some(0));
    assert_eq!(cassette.pick(&tools, &[0]), Some(1));
    assert_eq!(cassette.pick(&tools, &[0, 1]), None);
    assert_eq!(cassette.pick(&plain, &[2, 2]), Some(2));
}
