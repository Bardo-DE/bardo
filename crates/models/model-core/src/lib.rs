//! One trait for the models behind every engine and account, local or cloud.
//!
//! A [`Provider`] is one endpoint (a running engine or a cloud account). A turn pushes events
//! into a `TurnSink`; cancellation is dropping the future. The data a turn is made of is in
//! `model-types`; this crate holds the seam over it, the retry wrapper and the check of a
//! conversation's order.
//!
//! ```
//! use model_core::{RetryPolicy, next_wait};
//! use model_types::{Attempt, Permille, RetryClass, WaitMs};
//!
//! // The first retry waits the base; a refusal is never retried.
//! let policy = RetryPolicy { attempts: Attempt(3), base: WaitMs(100), cap: WaitMs(1_000) };
//! let wait = next_wait(&policy, Attempt(1), RetryClass::Transient, Permille(0));
//! assert_eq!(wait, Some(WaitMs(100)));
//! assert_eq!(next_wait(&policy, Attempt(1), RetryClass::Never, Permille(0)), None);
//! ```

mod embed;
mod provider;
mod retry;
mod sequence;

pub use embed::{Embedder, plan_batches};
pub use provider::Provider;
pub use retry::{RetryPolicy, Retrying, next_wait};
pub use sequence::{SequenceError, check as check_sequence};
