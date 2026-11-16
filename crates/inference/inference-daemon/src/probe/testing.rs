//! Values the unit tests of the probe, the book, the report and the watch share.

use super::{Found, Probed, Runtime};
use account_capability::Modality;
use account_capability::ModelId;
use account_capability::{Capability, EmbedCap, EmbedPrompts, LlmCap, LlmFeature, LlmWire};
use account_capability::{Claim, Offer, Provenance, Subject};
use account_ids::{Count, Dims, Tokens};
use account_provider::Port;

fn claim(id: &ModelId, capability: Capability) -> Claim {
    Claim {
        subject: Subject::Model(id.clone()),
        offer: Offer::Present(capability),
        provenance: Provenance::Discovered,
    }
}

/// A chat model with these features, as a probe reads it from Ollama (its native wire).
pub fn chat(id: &str, name: &str, context: u32, features: &[LlmFeature]) -> Probed {
    let model = ModelId::parse(id).expect("id");
    Probed {
        claims: vec![claim(
            &model,
            Capability::Llm(LlmCap {
                features: features.iter().copied().collect(),
                context: Tokens(context),
                max_output: Tokens(context),
                wire: LlmWire::OllamaNative,
            }),
        )],
        id: model,
        name: name.to_owned(),
    }
}

/// An embedding model.
pub fn embedding(id: &str, name: &str) -> Probed {
    let model = ModelId::parse(id).expect("id");
    Probed {
        claims: vec![claim(
            &model,
            Capability::Embeddings(EmbedCap {
                dims: Dims(768),
                modalities: [Modality::Text].into(),
                max_input: Tokens(2048),
                max_batch: Count(1),
                prompts: Box::new(EmbedPrompts {
                    query: "search_query: ".to_owned(),
                    document: String::new(),
                }),
            }),
        )],
        id: model,
        name: name.to_owned(),
    }
}

/// A runtime on `port` serving `models`.
pub fn found(runtime: Runtime, port: u16, models: Vec<Probed>) -> Found {
    Found {
        runtime,
        port: Port(port),
        models,
    }
}
