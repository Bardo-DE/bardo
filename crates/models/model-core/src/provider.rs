//! The provider trait.

use model_types::{Charges, ModelInfo, ProviderError, TurnEnd, TurnRequest, TurnSink};

/// One endpoint: a running engine or a cloud account.
///
/// Cancellation is drop: the future is cancel-safe, and dropping it closes the connection, which
/// aborts generation in vLLM and llama-server. A sink answering `ControlFlow::Break(())` ends the turn early
/// with `StopReason::EndTurn`.
pub trait Provider: Send + Sync {
    /// The models this endpoint serves (`/v1/models`, `/props`).
    fn describe(&self) -> impl Future<Output = Result<Vec<ModelInfo>, ProviderError>> + Send;

    /// Runs one turn: pushes its events into `sink` and returns how it ended.
    fn turn<K: TurnSink>(
        &self,
        request: &TurnRequest,
        sink: &mut K,
    ) -> impl Future<Output = Result<TurnEnd, ProviderError>> + Send;

    /// Whether this endpoint reports what a reply cost. Only an endpoint that does overrides it.
    fn charges(&self) -> Charges {
        Charges::NotReported
    }
}
