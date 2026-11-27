//! How a failure may be retried.

use serde::{Deserialize, Serialize};

use crate::{ProviderError, RetrySeconds};

/// How a failure may be retried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum RetryClass {
    Never,
    /// A transient fault: try again after the backoff.
    Transient,
    /// The server is overloaded and said how long to wait.
    Overload(RetrySeconds),
}

impl ProviderError {
    /// 408, 425, 429 and 5xx are retryable (`Server`, `RateLimited`); a refusal never is; an
    /// unreachable or not-ready engine and a timeout are transient; the rest are not.
    #[must_use]
    pub fn retry_class(&self) -> RetryClass {
        match self {
            ProviderError::RateLimited(wait) => RetryClass::Overload(*wait),
            ProviderError::Server(status) if retryable_status(status.0) => RetryClass::Transient,
            ProviderError::Unreachable | ProviderError::NotReady | ProviderError::Timeout => {
                RetryClass::Transient
            }
            ProviderError::Server(_)
            | ProviderError::Unauthorized
            | ProviderError::AuthRejected(_)
            | ProviderError::PaymentRequired(_)
            | ProviderError::ContextOverflow { .. }
            | ProviderError::BadRequest(_)
            | ProviderError::Refused(_)
            | ProviderError::Unreadable(_) => RetryClass::Never,
        }
    }
}

/// 408 (request timeout), 425 (too early), 429 (too many requests) and every 5xx.
fn retryable_status(status: u16) -> bool {
    matches!(status, 408 | 425 | 429 | 500..=599)
}
