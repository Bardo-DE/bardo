//! Embeddings through a provider: the role of a text, the request, the reply and the check of a
//! reply.
//!
//! Asymmetric models (nomic, e5, Qwen3-embedding) give worse recall, silently, when queries and
//! documents are embedded alike, so the role is part of the request and the prefix belongs to the
//! model (`EmbedPrompts`), not to the caller: an index and its queries cannot disagree.

use core::fmt;

use serde::{Deserialize, Serialize};

use crate::{BatchMax, Knob, ModelName, TurnUsage};
use base_ids::{Count, Dims, Tokens};

/// What a text is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbedRole {
    /// A search query.
    Query,
    /// A passage that is indexed.
    Document,
}

/// Text put in front of an input of one role (`search_query: `); empty for a symmetric model.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PrefixText(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EmbedPrompts {
    pub query: PrefixText,
    pub document: PrefixText,
}

/// What an embedding model can do.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EmbedCaps {
    pub dims: Dims,
    pub max_batch: BatchMax,
    pub max_input: Tokens,
    pub prompts: EmbedPrompts,
}

/// One embedding call.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedTurn {
    pub model: ModelName,
    /// The texts (what the person wrote: `Debug` shows the count only).
    pub inputs: Vec<String>,
    pub role: EmbedRole,
    /// The width to ask for. Servers that ignore a `dimensions` field (llama.cpp) are never sent
    /// one; the width then comes from the catalog.
    pub dims: Knob<Dims>,
}

impl fmt::Debug for EmbedTurn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EmbedTurn")
            .field("model", &self.model)
            .field("inputs", &format_args!("<{} texts>", self.inputs.len()))
            .field("role", &self.role)
            .field("dims", &self.dims)
            .finish()
    }
}

/// One embedding vector. Floats, because embeddings are floats end to end; so `PartialEq` only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EmbedVector(pub Vec<f32>);

/// The answer: one vector per input, in input order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbedEnd {
    pub vectors: Vec<EmbedVector>,
    pub usage: TurnUsage,
    pub served: ModelName,
}

/// A reply that is not the shape that was asked for. Checked before a vector reaches a store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum EmbedError {
    #[error("asked for {want:?} vectors, got {got:?}")]
    CountMismatch { want: Count, got: Count },
    #[error("expected vectors of width {want:?}, got {got:?}")]
    WidthMismatch { want: Dims, got: Dims },
}

impl EmbedEnd {
    /// `Ok` when there are `want` vectors, each `dims` wide.
    ///
    /// # Errors
    ///
    /// Returns [`EmbedError::CountMismatch`] when there are not `want` vectors and
    /// [`EmbedError::WidthMismatch`] when one is not `dims` wide.
    pub fn check(&self, want: Count, dims: Dims) -> Result<(), EmbedError> {
        let got = Count(u32::try_from(self.vectors.len()).unwrap_or(u32::MAX));
        if got != want {
            return Err(EmbedError::CountMismatch { want, got });
        }
        self.vectors
            .iter()
            .map(|vector| Dims(u32::try_from(vector.0.len()).unwrap_or(u32::MAX)))
            .find(|width| *width != dims)
            .map_or(Ok(()), |got| {
                Err(EmbedError::WidthMismatch { want: dims, got })
            })
    }
}
