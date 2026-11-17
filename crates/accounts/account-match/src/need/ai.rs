//! Needs for the AI kinds. Sets are subsets the offer must contain; numbers are minimums.
//!
//! Each need grows by adding a minimum, so each is `#[non_exhaustive]` and built with `new`.

use crate::short::{Shortfall, Wants, first_short};
use account_capability::{
    ComputerUseCap, ComputerUseEnv, EmbedCap, ImageGenCap, ImageMode, LlmCap, LlmFeature, Modality,
    RerankCap, SpeechCap, SpeechMode,
};
use base_ids::{Count, Dims, Px, Tokens};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A language model.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LlmNeed {
    /// Features it must have.
    pub features: BTreeSet<LlmFeature>,
    /// At least this context window.
    pub context: Tokens,
}

impl LlmNeed {
    /// A language model with these features and at least this context window.
    #[must_use]
    pub fn new(features: BTreeSet<LlmFeature>, context: Tokens) -> Self {
        Self { features, context }
    }
}

/// What an app asks when it wants "AI access", never one key: a language model that can chat, with
/// any context window.
#[must_use]
pub fn llm_need() -> crate::Need {
    crate::Need::Llm(LlmNeed::new([LlmFeature::Chat].into(), Tokens(0)))
}

/// An embedding model.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EmbedNeed {
    /// The vector length an index was built with, or any.
    pub dims: DimsNeed,
    /// Inputs it must embed.
    pub modalities: BTreeSet<Modality>,
}

impl EmbedNeed {
    /// An embedding model with this vector length that embeds these inputs.
    #[must_use]
    pub fn new(dims: DimsNeed, modalities: BTreeSet<Modality>) -> Self {
        Self { dims, modalities }
    }
}

/// Which vector length an embedding need accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum DimsNeed {
    /// Any length (a fresh index).
    Any,
    /// Exactly this length (an existing index).
    Exactly(Dims),
}

/// A speech model.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpeechNeed {
    /// Modes it must have.
    pub modes: BTreeSet<SpeechMode>,
}

impl SpeechNeed {
    /// A speech model with these modes.
    #[must_use]
    pub fn new(modes: BTreeSet<SpeechMode>) -> Self {
        Self { modes }
    }
}

/// An image generator.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImageGenNeed {
    /// Modes it must have.
    pub modes: BTreeSet<ImageMode>,
    /// Images at least this large on their longest side.
    pub max_side: Px,
}

impl ImageGenNeed {
    /// An image generator with these modes and images at least this large.
    #[must_use]
    pub fn new(modes: BTreeSet<ImageMode>, max_side: Px) -> Self {
        Self { modes, max_side }
    }
}

/// A reranker.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RerankNeed {
    /// At least this many documents per call.
    pub max_docs: Count,
}

impl RerankNeed {
    /// A reranker that takes at least this many documents per call.
    #[must_use]
    pub fn new(max_docs: Count) -> Self {
        Self { max_docs }
    }
}

/// A computer-use model. Only the environments are asked for; protocol fields (batching, wire,
/// image size) are the engine's business.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ComputerUseNeed {
    /// Environments it must operate.
    pub environments: BTreeSet<ComputerUseEnv>,
}

impl ComputerUseNeed {
    /// A computer-use model that operates these environments.
    #[must_use]
    pub fn new(environments: BTreeSet<ComputerUseEnv>) -> Self {
        Self { environments }
    }
}

impl Wants<LlmCap> for LlmNeed {
    fn short_of(&self, offer: &LlmCap) -> Option<Shortfall> {
        first_short(&[
            (
                self.features.is_subset(&offer.features),
                Shortfall::Features,
            ),
            // A window of zero is one nobody stated (a server of the person's own lists its
            // models and no limits): it is not known to be too small.
            (
                offer.context.0 == 0 || offer.context >= self.context,
                Shortfall::Context,
            ),
        ])
    }
}

impl Wants<EmbedCap> for EmbedNeed {
    fn short_of(&self, offer: &EmbedCap) -> Option<Shortfall> {
        first_short(&[
            (
                matches!(self.dims, DimsNeed::Any) || self.dims == DimsNeed::Exactly(offer.dims),
                Shortfall::Dims,
            ),
            (
                self.modalities.is_subset(&offer.modalities),
                Shortfall::Modalities,
            ),
        ])
    }
}

impl Wants<SpeechCap> for SpeechNeed {
    fn short_of(&self, offer: &SpeechCap) -> Option<Shortfall> {
        first_short(&[(self.modes.is_subset(&offer.modes), Shortfall::Modes)])
    }
}

impl Wants<ImageGenCap> for ImageGenNeed {
    fn short_of(&self, offer: &ImageGenCap) -> Option<Shortfall> {
        first_short(&[
            (self.modes.is_subset(&offer.modes), Shortfall::Modes),
            (offer.max_side >= self.max_side, Shortfall::MaxSide),
        ])
    }
}

impl Wants<RerankCap> for RerankNeed {
    fn short_of(&self, offer: &RerankCap) -> Option<Shortfall> {
        first_short(&[(offer.max_docs >= self.max_docs, Shortfall::MaxDocs)])
    }
}

impl Wants<ComputerUseCap> for ComputerUseNeed {
    fn short_of(&self, offer: &ComputerUseCap) -> Option<Shortfall> {
        first_short(&[(
            self.environments.is_subset(&offer.environments),
            Shortfall::Environments,
        )])
    }
}
