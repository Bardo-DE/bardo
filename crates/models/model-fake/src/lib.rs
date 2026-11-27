//! Fakes of the model layer, for tests.
//!
//! [`ScriptedProvider`] plays scripted turns and keeps every request it was given, so a test can
//! assert on prompt assembly. [`stream`] runs reply bytes through the same framer and decoder the
//! OpenAI-compatible driver uses, and checks what must hold of any reply. A crate lists this one
//! under `[dev-dependencies]` only.
//!
//! ```
//! use model_fake::{Script, ScriptedProvider};
//!
//! // A provider that has seen no turn has recorded no request.
//! let provider = ScriptedProvider::new(vec![], Vec::<Script>::new());
//! assert!(provider.requests().is_empty());
//! ```

mod scripted;
pub mod stream;

pub use scripted::{Script, ScriptedProvider};
