//! What went wrong while Cedar ruled on a request. A fault never permits: the ruling that comes
//! with it is the stricter reading of the query that faulted.

use permission_core::Ruling;
use std::error::Error;
use std::sync::Arc;

/// Why one query could not be answered.
#[derive(Debug, Clone, thiserror::Error)]
pub enum FaultCause {
    /// A principal, app or action name is not a Cedar name.
    #[error("a name of the request is not a Cedar name")]
    Name,
    /// The entities do not fit the schema.
    #[error("the request's entities do not fit the schema")]
    Entities(#[source] Arc<dyn Error + Send + Sync>),
    /// The query's action is not a Cedar name.
    #[error("the query's action is not a Cedar name")]
    Action,
    /// The context does not fit the schema.
    #[error("the request's context does not fit the schema")]
    Context(#[source] Arc<dyn Error + Send + Sync>),
    /// Cedar refused the request as a whole.
    #[error("Cedar refused the request")]
    Request(#[source] Arc<dyn Error + Send + Sync>),
    /// This many policies raised an evaluation error (a policy that errors is skipped; this
    /// includes Cedar refusing to evaluate on a stack that is too small).
    #[error("{0} policies failed to evaluate")]
    Policies(usize),
}

/// One query that failed: which, and why.
#[derive(Debug, Clone, thiserror::Error)]
#[error("query `{query}`")]
pub struct EvalFault {
    /// The Cedar action of the query (`perform`, `perform_unasked`, ...).
    pub query: &'static str,
    /// What failed.
    #[source]
    pub cause: FaultCause,
}

/// A ruling and the faults met reaching it. Empty `faults` is the ordinary case.
#[derive(Debug, Clone)]
pub struct Ruled {
    /// The ruling, fail-safe: a faulted query counts as not permitted.
    pub ruling: Ruling,
    /// Every query that faulted, in the order asked.
    pub faults: Vec<EvalFault>,
}
