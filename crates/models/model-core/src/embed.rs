//! The embedding seam and the batch plan around it.

use core::ops::Range;

use model_types::{BatchMax, EmbedEnd, EmbedTurn, ProviderError};

/// An embedding backend. Cancellation is drop.
pub trait Embedder: Send + Sync {
    /// Embeds the texts of `turn`: one vector per input, in order.
    fn embed(
        &self,
        turn: &EmbedTurn,
    ) -> impl Future<Output = Result<EmbedEnd, ProviderError>> + Send;
}

/// The ranges of `n` inputs, in order, each at most `max` long (a `max` of zero counts as one).
///
/// Callers send one range per call and reassemble the vectors by range, so an index rebuild
/// issues the same calls every time.
#[must_use]
pub fn plan_batches(n: usize, max: BatchMax) -> Vec<Range<usize>> {
    let step = usize::try_from(max.0).unwrap_or(usize::MAX).max(1);
    (0..n)
        .step_by(step)
        .map(|start| start..start.saturating_add(step).min(n))
        .collect()
}
