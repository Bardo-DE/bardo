//! What inferd tells accountd of a local runtime.
//!
//! What inferd tells accountd of a local runtime: the runtime is an account (Ollama, llama.cpp,
//! LM Studio; design/31 section 3.3) that Settings lists and that goes `Offline`, never gone, when
//! the runtime stops. It carries no secret: the provider's id, the models as `Discovered` claims,
//! and a state. The report is a call on accountd's `Broker` (`broker::AccountdBroker`).
//!
//! [`Reports`] is the seam: accountd in the daemon, a recorder in a unit test.

use crate::cloud::accountd::Boxed;
use crate::probe::Runtime;
use crate::probed::Standing;
use account_capability::Claim;
use account_core::AccountId;
use std::error::Error;
use std::fmt::Debug;
use std::sync::Arc;

/// Why a report was not taken.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ReportError {
    /// There is no store to report to: ask again at the next look.
    #[error("there is no store to report to")]
    Unreachable,
    /// The store could not take it now: ask again at the next look.
    #[error("the store could not take the report now")]
    Unavailable(#[source] Arc<dyn Error + Send + Sync>),
    /// The store did not accept it (an unknown provider, a claim it does not accept).
    #[error("the store did not accept the report")]
    Rejected(#[source] Arc<dyn Error + Send + Sync>),
}

/// accountd, as inferd reports a runtime to it.
pub trait Reports: Debug + Send + Sync + 'static {
    /// Reports `runtime` in `standing` with its models as `claims`; the id of its account.
    fn report<'a>(
        &'a self,
        runtime: Runtime,
        claims: &'a [Claim],
        standing: Standing,
    ) -> Boxed<'a, Result<AccountId, ReportError>>;
}

/// No accountd: every report is `Unreachable`, so the daemon serves its probed runtimes on its
/// own.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoReports;

impl Reports for NoReports {
    fn report<'a>(
        &'a self,
        _: Runtime,
        _: &'a [Claim],
        _: Standing,
    ) -> Boxed<'a, Result<AccountId, ReportError>> {
        Box::pin(async { Err(ReportError::Unreachable) })
    }
}

#[cfg(test)]
mod tests;
