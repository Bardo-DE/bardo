//! The local model one probed model is: a catalog entry written from what the probe read (as the
//! replay engine's is), so routing sees a card like any other and the turn code finds the
//! sampling, limits and tools it reads from an entry. Nothing starts: the model is served by the
//! runtime on its loopback port, its engine id names no process, and its card says `OnDevice`.

use super::{Probed, Runtime};
use engine_spec::{EnginePaths, ProgramPath};
use engine_supervisor::{EngineId, EngineSpec, SocketPath, command};
use model_catalog::{EngineKind, MiB};
use model_http::Port;
use model_openai_compat::Flavor;
use model_provider::ModelName;
use account_capability::Offer;
use account_capability::{Billing, Locality};
use account_capability::{Capability, EmbedCap, LlmCap, LlmFeature, LlmWire};
use account_core::AccountId;
use inference_core::ModelCard;
use inference_router::catalog::parse_entry_text;
use inference_router::local::LocalModel;
use std::path::{Path, PathBuf};

const SAMPLING: &str = r#"sampling = { reasoning_on = { temperature = 600, top_p = { kind = "off" }, top_k = { kind = "off" }, min_p = { kind = "off" }, repeat_penalty = { kind = "off" }, seed = { kind = "off" } }, reasoning_default = "off", reasoning_off = { temperature = 0, top_p = { kind = "off" }, top_k = { kind = "off" }, min_p = { kind = "off" }, repeat_penalty = { kind = "off" }, seed = { kind = "off" } } }"#;

/// A TOML string, quoted.
fn quoted(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_owned())
}

/// What the probe read of a model's chat side, as the catalog's tables.
fn chat_tables(cap: &LlmCap) -> String {
    let has = |feature| cap.features.contains(&feature);
    let tools = match has(LlmFeature::Tools) {
        true => "server_parsed",
        false => "absent",
    };
    let structured = match has(LlmFeature::StructuredOutput) {
        true => r#"["json_schema"]"#,
        false => "[]",
    };
    let reasoning = match has(LlmFeature::Reasoning) {
        true => "present",
        false => "absent",
    };
    let mut tables = format!(
        r#"text_out = {{ tools = "{tools}", structured = {structured}, reasoning = "{reasoning}", streaming = "present", context = {}, max_output = {}, {SAMPLING} }}
"#,
        cap.context.0, cap.max_output.0
    );
    if has(LlmFeature::Vision) {
        tables.push_str(
            r#"image_in = { per_prompt = 1, rule = { kind = "identity" }, space = { kind = "image" } }
"#,
        );
    }
    tables
}

fn embed_table(cap: &EmbedCap) -> String {
    format!(
        "vector_out = {{ dims = {}, max_batch = {}, max_input = {}, prompts = {{ query = {}, document = {} }} }}\n",
        cap.dims.0,
        cap.max_batch.0.max(1),
        cap.max_input.0,
        quoted(&cap.prompts.query),
        quoted(&cap.prompts.document),
    )
}

/// The catalog text of a probed model.
fn entry_text(model: &Probed, llm: Option<&LlmCap>, embed: Option<&EmbedCap>) -> String {
    let sees = llm.is_some_and(|cap| cap.features.contains(&LlmFeature::Vision));
    let inputs = if sees {
        r#"["text", "image"]"#
    } else {
        r#"["text"]"#
    };
    let outputs: Vec<&str> = [llm.map(|_| "\"text\""), embed.map(|_| "\"vector\"")]
        .into_iter()
        .flatten()
        .collect();
    let mut text = format!(
        r#"id = {}
label = {}
licence = {{ kind = "open", v = "unknown" }}
family = "probed"
cold_start_estimate_s = 0
source = {{ kind = "hugging_face", v = {{ repo = "probed/{}", revision = "0000000000000000000000000000000000000000" }} }}
vram = {{ weights_mib = 0, kv_per_1k_ctx_mib = 0, overhead_mib = 0 }}
inputs = {inputs}
outputs = [{}]
"#,
        quoted(model.id.as_str()),
        quoted(&model.name),
        model.id.as_str(),
        outputs.join(", "),
    );
    text.push_str(&llm.map(chat_tables).unwrap_or_default());
    text.push_str(&embed.map(embed_table).unwrap_or_default());
    text.push_str(
        "\n[[engine]]\nkind = \"llama_server\"\nargs = []\nweights = { kind = \"gguf\", v = { model = \"probed.gguf\" } }\n",
    );
    text
}

/// What a runtime's claims say a model can do, as the card lists it: the chat wire is the one
/// spoken to every runtime here (OpenAI-compatible chat completions), whatever the probe's
/// source said.
fn capabilities_of(model: &Probed) -> Vec<Capability> {
    model
        .claims
        .iter()
        .filter_map(|claim| match &claim.offer {
            Offer::Present(Capability::Llm(cap)) => Some(Capability::Llm(LlmCap {
                wire: LlmWire::ChatCompletions,
                ..cap.clone()
            })),
            Offer::Present(capability) => Some(capability.clone()),
            Offer::Absent { .. } => None,
        })
        .collect()
}

/// The local model of one probed model of `runtime`, on `port`. `sockets` is where an engine's
/// socket would go (the field is required of every model and unused here). `None` when the model
/// is neither a language nor an embedding model, or its entry does not parse.
pub fn local_model(
    runtime: Runtime,
    port: account_provider::Port,
    model: &Probed,
    sockets: &Path,
) -> Option<LocalModel> {
    let capabilities = capabilities_of(model);
    let llm = capabilities.iter().find_map(|c| match c {
        Capability::Llm(cap) => Some(cap),
        _ => None,
    });
    let embed = capabilities.iter().find_map(|c| match c {
        Capability::Embeddings(cap) => Some(cap),
        _ => None,
    });
    if llm.is_none() && embed.is_none() {
        return None;
    }
    let entry = parse_entry_text(&entry_text(model, llm, embed)).ok()?;
    let profile = entry.engines.first()?.clone();
    let provider = runtime.provider();
    let socket = SocketPath(sockets.join(format!("probed-{provider}-{}.sock", model.id.as_str())));
    let paths = EnginePaths::new(PathBuf::new()).with_program(
        EngineKind::LlamaServer,
        ProgramPath(PathBuf::from("probed")),
    );
    Some(LocalModel {
        card: ModelCard {
            account: AccountId::parse(provider).ok()?,
            model: model.id.clone(),
            locality: Locality::OnDevice,
            billing: Billing::Free,
            capabilities,
        },
        name: ModelName(model.name.clone()),
        spec: EngineSpec {
            id: EngineId(format!("probed:{provider}:{}", model.id.as_str())),
            need: MiB(0),
            unit: command(&entry, &profile, &paths, &socket).ok()?,
        },
        socket,
        flavor: Some(Flavor::LlamaServer),
        cassette: None,
        loopback: Some(Port(port.0)),
        attached: None,
        entry,
        profile,
    })
}
