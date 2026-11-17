//! The computer-use eval against a hosted model, the way a hosted model is reached from this
//! computer: through the accounts service's inferd on the bus. The API key is held by accountd and used by
//! inferd for one turn; this process never has it, and says so ([`isolation`]).
//!
//! `inferd` refuses a computer-use session on a hosted model today (the accounts service FINDINGS, "inferd
//! hosted"), so this does what computer-use-system's `InferdComputerUseModel` does for a local one, one layer up:
//! The model layer's `ComputerUseSession` (run by `computer_use_eval::live::ProviderModel`) builds each step's prompt and
//! reads the reply, and the one model turn in between is a chat turn with the screenshot inline,
//! asked of inferd as `Need::Llm` on data class `Screen` ([`provider::InferdProvider`]).

#[cfg(test)]
mod entry;
pub mod error;
pub mod isolation;
pub mod provider;
pub mod request;
pub mod watch;
