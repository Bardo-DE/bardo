//! The shipped catalogue: every file is listed, parses and keeps the invariants.

use std::path::PathBuf;

use model_catalog::{ContextArg, EngineKind, Family, ModelEntry, parse_entry};

const IDS: [&str; 10] = [
    "holo-3.1-4b",
    "qwen3-4b-instruct-2507-fp8",
    "granite-4.2-3b-fp8",
    "qwen3-8b-awq",
    "qwen3.5-35b-a3b-fp8",
    "nemotron-3.5-asr-streaming",
    "kokoro-82m",
    "whisper-large-v3",
    "whisper-large-v3-turbo",
    "breeze-asr-25",
];

/// The curated hosted entries, in catalogue order.
const REMOTE: [&str; 8] = [
    "claude-opus-5.5",
    "claude-haiku-4.5",
    "gemini-3.1-pro",
    "gemini-3.8-flash",
    "kimi-k3",
    "kimi-k2.6",
    "gpt-6-astra",
    "gpt-6-luna",
];

/// The hosted entries reached through `OpenRouter` and the provider's own API, checked below with
/// their own asserts: `DeepSeek` V4 Pro (text only) and Muse Spark 1.3 contributor (one reach).
const MORE_HOSTED: [&str; 2] = ["deepseek-v4-pro", "muse-spark-1.3-contributor"];

/// The Holo ladder above 4B: local computer-use entries with estimated vram, checked on their own.
const HOLO_LADDER: [&str; 2] = ["holo-3.1-9b", "holo4-35b-a3b-fp8"];

/// Local computer-use entries that speak the Qwen tool dialect through an action profile.
const PROFILED: [&str; 1] = ["ui-mate-27b"];

fn read(dir: &str, id: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(dir)
        .join(format!("{id}.toml"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn shipped(id: &str) -> ModelEntry {
    parse_entry(&read("../catalog", id)).unwrap_or_else(|e| panic!("{id}: {e}"))
}

#[test]
fn every_shipped_file_is_listed_here_and_has_its_id_as_stem() {
    let mut found: Vec<String> =
        std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../catalog"))
            .unwrap()
            .map(|f| {
                f.unwrap()
                    .path()
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
    found.sort();
    let mut listed: Vec<String> = IDS
        .iter()
        .chain(&REMOTE)
        .chain(&MORE_HOSTED)
        .chain(&HOLO_LADDER)
        .chain(&PROFILED)
        .map(|s| (*s).to_owned())
        .collect();
    listed.sort();
    assert_eq!(found, listed);
    for id in IDS
        .iter()
        .chain(&REMOTE)
        .chain(&MORE_HOSTED)
        .chain(&HOLO_LADDER)
        .chain(&PROFILED)
    {
        assert_eq!(&shipped(id).id.0, id);
    }
}

#[test]
fn every_entry_names_its_family() {
    for (id, family) in [
        ("holo-3.1-4b", "qwen"),
        ("qwen3-4b-instruct-2507-fp8", "qwen"),
        ("granite-4.2-3b-fp8", "granite"),
        ("qwen3-8b-awq", "qwen"),
        ("qwen3.5-35b-a3b-fp8", "qwen"),
        ("kokoro-82m", "kokoro"),
        ("whisper-large-v3", "whisper"),
        ("whisper-large-v3-turbo", "whisper"),
        ("breeze-asr-25", "whisper"),
        ("nemotron-3.5-asr-streaming", "nemotron"),
    ] {
        assert_eq!(shipped(id).family, Family(family.into()), "{id}");
    }
}

#[test]
fn every_client_parsed_entry_serves_without_a_server_parser() {
    use model_types::ToolSupport;
    let client: Vec<&str> = IDS
        .iter()
        .chain(PROFILED.iter())
        .copied()
        .filter(|id| {
            let text = shipped(id).capabilities.text_out;
            text.is_some_and(|text| text.tools == ToolSupport::ClientParsed)
        })
        .collect();
    assert_eq!(client, ["ui-mate-27b"]);
    for id in client {
        for engine in &shipped(id).engines {
            let parsers = engine.args.iter().filter(|a| a.0.contains("parser"));
            assert_eq!(parsers.count(), 0, "{id}");
        }
    }
}

#[test]
fn a_model_without_json_schema_declares_json_object() {
    use model_types::Constraint;
    let text = |id| shipped(id).capabilities.text_out.unwrap().structured;
    assert_eq!(text("deepseek-v4-pro"), [Constraint::JsonObject].into());
    assert_eq!(text("holo-3.1-4b"), [Constraint::JsonSchema].into());
}

#[test]
fn every_vllm_line_names_its_context_arg_and_the_limits_agree() {
    let ids: Vec<String> = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../catalog"))
        .unwrap()
        .map(|f| {
            let name = f.unwrap().file_name().to_string_lossy().into_owned();
            name.trim_end_matches(".toml").to_owned()
        })
        .collect();
    for id in ids {
        let entry = shipped(&id);
        let Some(text) = entry.capabilities.text_out.as_ref() else {
            continue;
        };
        // A model that transcribes has no chat window to serve or to state limits for.
        if text.context.0 > 0 {
            for engine in entry.engines.iter().filter(|e| e.kind == EngineKind::Vllm) {
                assert!(
                    matches!(&engine.context_arg, Some(ContextArg::Flag(_))),
                    "{id}: a vLLM line names its context arg"
                );
                assert!(
                    engine.args.iter().all(|a| a.0 != "--max-model-len"),
                    "{id}: the window is not hard-coded in the args"
                );
            }
            assert!(text.context_max.is_some(), "{id}: states context_max");
        }
        if let Some(max) = text.context_max {
            assert!(max >= text.context, "{id}: context_max >= context");
        }
        if let Some(max) = text.max_output_max {
            assert!(max >= text.max_output, "{id}: max_output_max >= max_output");
        }
        if let (Some(min), Some(max)) = (text.context_min, text.context_max) {
            assert!(min <= max, "{id}: context_min <= context_max");
        }
    }
}
