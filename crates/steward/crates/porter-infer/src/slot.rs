//! Slots: the jobs a person picks a model for (capabilities.md section 2). A slot is a capability
//! signature; any catalogue entry that satisfies it is listed there, so one model may sit in
//! several slots. The settings key is `ai.model.<slot>.<tier>`.

use porter_capability::Tier;
use serde::{Deserialize, Serialize};

/// The jobs a user maps models to. The slugs are the serde form and the settings-key segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Slot {
    /// Text in, text out (with tools): language models. Was `AiKind::Llm`.
    Text,
    /// Audio in, text out: speech to text. Was `AiKind::SpeechIn`.
    VoiceIn,
    /// Text in, audio out: text to speech. Was `AiKind::SpeechOut`.
    VoiceOut,
    /// Image in, text out: a model that reads or describes pictures.
    ImageIn,
    /// Image in, actions out: computer-use models.
    ComputerUse,
    /// Text in, vector out: embedding models.
    Embeddings,
    /// Image generators.
    ImageGen,
    /// Rerankers.
    Rerank,
}

impl Slot {
    /// Every slot, in the order the picker lists them.
    pub const ALL: [Slot; 8] = [
        Slot::Text,
        Slot::VoiceIn,
        Slot::VoiceOut,
        Slot::ImageIn,
        Slot::ComputerUse,
        Slot::Embeddings,
        Slot::ImageGen,
        Slot::Rerank,
    ];

    /// The slot's stable slug: its serde form and its settings-key segment.
    pub fn slug(self) -> &'static str {
        match self {
            Slot::Text => "text",
            Slot::VoiceIn => "voice_in",
            Slot::VoiceOut => "voice_out",
            Slot::ImageIn => "image_in",
            Slot::ComputerUse => "computer_use",
            Slot::Embeddings => "embeddings",
            Slot::ImageGen => "image_gen",
            Slot::Rerank => "rerank",
        }
    }

    /// The slot a slug names.
    pub fn from_slug(slug: &str) -> Option<Slot> {
        Slot::ALL.into_iter().find(|slot| slot.slug() == slug)
    }

    /// The settings key for the model chosen at `tier` (`ai.model.voice_in.balanced`).
    pub fn setting_key(self, tier: Tier) -> String {
        let tier = match tier {
            Tier::Fast => "fast",
            Tier::Balanced => "balanced",
            Tier::Best => "best",
        };
        format!("ai.model.{}.{tier}", self.slug())
    }
}
