//! The model catalog as the router sees it.
//!
//! The catalog's `catalog/*.toml` entries become `Claim`s at `Provenance::Curated` for the `local`
//! provider (`Discovery::Supervised`). The parsing and the merge are `model-catalog`'s; this module
//! maps an entry's capabilities to the router's vocabulary. Reading the directories is
//! `inference-fs`'s (`read_catalog`); this module holds the value they produce ([`Catalog`]).
//!
//! An entry that gives vectors makes a claim from its `vector_out` table (the vector length, the
//! batch and input limits, the query and document prefixes). inferd keeps no override of its own: the catalog is
//! the one place a model's details are written, and a user who needs other prefixes writes a
//! catalog file of the same id in `$XDG_DATA_HOME/bardo/models/catalog`.

use account_capability::ModelId;
use account_capability::{
    Capability, CuaBatching, CuaCap, CuaEnv, EmbedCap, EmbedPrompts, LanguageSet, LanguageTag,
    LlmCap, LlmFeature, LlmWire, Modality, SpeechCap, SpeechMode,
};
use account_capability::{Claim, Offer, Provenance, Subject};
use base_ids::{Count, Px};
use computer_use_action::CuaDialect;
use engine_spec::{SpecError, check_entry, engine};
use model_catalog::{CatalogError, Modality as CatalogModality, ModelEntry, parse_entry};
use model_provider::{
    Caps, Constraint, CuaSupport, EmbedCaps, InputKind, Support, ToolSupport, Zoom,
};
use model_vision_prep::ResizeRule;
use speech_provider::{LangSet, SpeechDir};
use std::collections::BTreeSet;
use std::path::PathBuf;

/// Where the catalog files live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogDirs {
    /// `/usr/share/bardo/models/catalog`.
    pub system: PathBuf,
    /// `$XDG_DATA_HOME/bardo/models/catalog`; a file here replaces the system file of the same id.
    pub user: PathBuf,
}

/// A catalog file that was not used, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    /// The file (or, for a bad id, the entry's id).
    pub what: String,
    /// The reason, as text for the log.
    pub why: String,
}

/// Every entry the directories give, and the files that were refused.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Catalog {
    /// The merged entries: system first, then user ids that are new.
    pub entries: Vec<ModelEntry>,
    /// Files that did not read or parse.
    pub skipped: Vec<Skipped>,
}

/// Why an entry's text is not an entry: it does not parse, or an engine it names cannot run it.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EntryError {
    /// Stoker's parser refused it.
    #[error("{0}")]
    Parse(#[from] CatalogError),
    /// It parses, but one of its engines cannot run it (`engine_spec::check_entry`).
    #[error("{0}")]
    Engine(#[from] SpecError),
}

/// One entry's text as an entry: stoker's parser, then the checks of the engines the entry names
/// (a vLLM line needs a GPU estimate, a llama-server line GGUF weights).
///
/// # Errors
/// `EntryError` when the text is not a valid entry, or an engine it names fails its checks.
pub fn parse_entry_text(text: &str) -> Result<ModelEntry, EntryError> {
    let entry = parse_entry(text)?;
    check_entry(&entry)?;
    Ok(entry)
}

/// The claims of one entry: a language model, a computer-use model, a speech model, an
/// embedding model with an `embed` table. An entry whose id is not a model id makes none.
#[must_use]
pub fn claims_of(entry: &ModelEntry) -> Vec<Claim> {
    let Ok(model) = ModelId::parse(&entry.id.0) else {
        return Vec::new();
    };
    capabilities_of(entry)
        .into_iter()
        .map(|capability| Claim {
            subject: Subject::Model(model.clone()),
            offer: Offer::Present(capability),
            provenance: Provenance::Curated,
        })
        .collect()
}

/// What the entry's capabilities make of it: a language model, a computer-use model, an embedding
/// model, a speech model.
///
/// A model with both audio tables keeps the transcription side only (its slots say the rest).
pub fn capabilities_of(entry: &ModelEntry) -> Vec<Capability> {
    let c = &entry.capabilities;
    let takes = |modality| c.inputs.contains(modality);
    let gives = |modality| c.outputs.contains(modality);
    let chat = c.chat_caps();
    let mut offered = Vec::new();
    if let Some(caps) = &chat {
        if takes(CatalogModality::Text) && gives(CatalogModality::Text) {
            offered.push(Capability::Llm(llm_cap(caps, entry)));
        }
        if takes(CatalogModality::Image) && gives(CatalogModality::Actions) {
            offered.extend(cua_cap(caps).map(Capability::ComputerUse));
        }
    }
    if takes(CatalogModality::Text) && gives(CatalogModality::Vector) {
        offered.extend(
            c.vector_out
                .as_ref()
                .map(|embed| Capability::Embeddings(embed_cap(embed))),
        );
    }
    let speech_in =
        takes(CatalogModality::Audio) && gives(CatalogModality::Text) && c.audio_in.is_some();
    let speech_out = takes(CatalogModality::Text)
        && gives(CatalogModality::Audio)
        && c.audio_out.is_some()
        && c.audio_in.is_none();
    if speech_in || speech_out {
        offered.extend(speech_cap(entry));
    }
    offered
}

fn llm_cap(caps: &Caps, entry: &ModelEntry) -> LlmCap {
    let caches_prompts = entry
        .engines
        .iter()
        .any(|profile| engine(profile.kind).caches_prompts());
    let features: [(bool, LlmFeature); 6] = [
        (true, LlmFeature::Chat),
        (caps.tools != ToolSupport::Absent, LlmFeature::Tools),
        (caps.inputs.contains(&InputKind::Image), LlmFeature::Vision),
        (
            // A model that takes only a JSON object still answers a shape: the schema goes in the
            // prompt and the reply is checked (model-extract).
            caps.output.contains(&Constraint::JsonSchema)
                || caps.output.contains(&Constraint::JsonObject),
            LlmFeature::StructuredOutput,
        ),
        (caps.reasoning == Support::Present, LlmFeature::Reasoning),
        (caches_prompts, LlmFeature::PromptCache),
    ];
    LlmCap {
        features: features
            .into_iter()
            .filter_map(|(has, feature)| has.then_some(feature))
            .collect(),
        context: caps.context,
        max_output: caps.max_output,
        wire: LlmWire::ChatCompletions,
    }
}

/// A computer-use claim needs a dialect an engine on this computer can speak (a tool call or the
/// text of UI-TARS: `computer-use-session` builds the prompt and reads the reply of both; a vendor's wire
/// needs the vendor's backend) and a model that sees images.
fn cua_cap(caps: &Caps) -> Option<CuaCap> {
    let CuaSupport::Dialect {
        dialect: CuaDialect::Tool(_) | CuaDialect::Text(_),
        batching,
        zoom,
        ..
    } = caps.computer_use
    else {
        return None;
    };
    Some(CuaCap {
        environments: BTreeSet::from([CuaEnv::Desktop]),
        batching: match batching {
            model_provider::Batching::One => CuaBatching::One,
            model_provider::Batching::Many => CuaBatching::Many,
        },
        zoom: match zoom {
            Zoom::Absent => false,
            Zoom::Native => true,
        },
        max_image: Px(longest_side(&caps.images.rule)),
        wire: LlmWire::ChatCompletions,
    })
}

/// The longest side the rule lets an image have: the cap's own, or the side of a square of the
/// largest area.
fn longest_side(rule: &ResizeRule) -> u32 {
    let square = |pixels: u64| u32::try_from(pixels.isqrt()).unwrap_or(u32::MAX);
    match rule {
        ResizeRule::SmartResize { max_pixels, .. } => square(max_pixels.0),
        ResizeRule::LongEdge { max_edge, .. } => max_edge.0,
        ResizeRule::Identity => 0,
    }
}

fn speech_cap(entry: &ModelEntry) -> Option<Capability> {
    let c = &entry.capabilities;
    let speech = c.audio_in.as_ref().or(c.audio_out.as_ref())?;
    let mode = match speech.dir() {
        SpeechDir::In => SpeechMode::Stt,
        SpeechDir::Out => SpeechMode::Tts,
    };
    let languages = match &speech.langs {
        LangSet::Any => LanguageSet::Any,
        LangSet::Listed(langs) => LanguageSet::Listed(
            langs
                .iter()
                .filter_map(|lang| LanguageTag::parse(lang.as_str()).ok())
                .collect(),
        ),
    };
    Some(Capability::Speech(SpeechCap {
        modes: BTreeSet::from([mode]),
        languages,
    }))
}

fn embed_cap(embed: &EmbedCaps) -> EmbedCap {
    EmbedCap {
        dims: embed.dims,
        modalities: BTreeSet::from([Modality::Text]),
        max_input: embed.max_input,
        max_batch: Count(embed.max_batch.0),
        prompts: Box::new(EmbedPrompts {
            query: embed.prompts.query.0.clone(),
            document: embed.prompts.document.0.clone(),
        }),
    }
}

#[cfg(test)]
mod tests;
