//! The model catalog as porter sees it: stoker's `catalog/*.toml` files, read from the system
//! and user directories, become `Claim`s at `Provenance::Curated` for the `local` provider
//! (`Discovery::Supervised`). The parsing and the merge are stoker's `model-catalog`; this module
//! reads the directories and maps an entry's capabilities to porter's vocabulary.
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
use base_ids::{Count, Dims, Px, Tokens};
use computer_use_action::CuaDialect;
use engine_spec::{SpecError, check_entry, engine};
use model_catalog::{
    CatalogError, Modality as CatalogModality, ModelEntry, merge_catalogs, parse_entry,
};
use model_provider::{
    Caps, Constraint, CuaSupport, EmbedCaps, InputKind, Support, ToolSupport, Zoom,
};
use model_vision_prep::ResizeRule;
use speech_provider::{LangSet, SpeechDir};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

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
pub fn parse_entry_text(text: &str) -> Result<ModelEntry, EntryError> {
    let entry = parse_entry(text)?;
    check_entry(&entry)?;
    Ok(entry)
}

/// Reads the system and user directories and merges them (a user file replaces the system file
/// of the same id). A missing directory is empty; a file that does not parse is skipped.
#[must_use]
pub fn read_catalog(dirs: &CatalogDirs) -> Catalog {
    let (system, mut skipped) = read_dir(&dirs.system);
    let (user, more) = read_dir(&dirs.user);
    skipped.extend(more);
    Catalog {
        entries: merge_catalogs(system, user),
        skipped,
    }
}

fn read_dir(dir: &Path) -> (Vec<ModelEntry>, Vec<Skipped>) {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    files.sort();
    let mut entries = Vec::new();
    let mut skipped = Vec::new();
    for path in files {
        let parsed = std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|text| parse_entry_text(&text).map_err(|e| e.to_string()));
        match parsed {
            Ok(entry) => entries.push(entry),
            Err(why) => skipped.push(Skipped {
                what: path.display().to_string(),
                why,
            }),
        }
    }
    (entries, skipped)
}

/// The claims the catalog makes for the local account: one per model and capability of an entry
/// that runs on this computer, at the provenance the catalog gives (`Curated`); a hosted entry is
/// not the local account's.
pub fn local_claims(dirs: &CatalogDirs) -> Vec<Claim> {
    read_catalog(dirs)
        .entries
        .iter()
        // An attached entry is the catalogue's word that an engine exists somewhere; where it is
        // is the person's (`where`), so it makes no claim until inferd attaches it, and then its
        // card says where the data goes.
        .filter(|entry| entry.locality.is_on_device() && entry.serving.is_launched())
        .flat_map(claims_of)
        .collect()
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

/// What the entry's capabilities make of it: a language model, a computer-use model, an
/// embedding model, a speech model. A model with both audio tables keeps the transcription
/// side only (its slots say the rest).
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
        context: Tokens(caps.context.0),
        max_output: Tokens(caps.max_output.0),
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
        dims: Dims(embed.dims.0),
        modalities: BTreeSet::from([Modality::Text]),
        max_input: Tokens(embed.max_input.0),
        max_batch: Count(embed.max_batch.0),
        prompts: Box::new(EmbedPrompts {
            query: embed.prompts.query.0.clone(),
            document: embed.prompts.document.0.clone(),
        }),
    }
}

#[cfg(test)]
mod tests;
