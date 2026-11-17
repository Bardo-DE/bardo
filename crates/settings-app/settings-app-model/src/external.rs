//! The types of other crates that the model's facts are made of.
//!
//! They are named once here so the window names only `settings-app-model` and
//! `settings-app-client`, never the memory service, the chord keymap, the actions service, porter or toml directly. Each is an id or a fact the model carries as it is; the window words it or
//! hands it back, and holds no rule of its own about it.

pub use account_capability::ModelId;
pub use account_core::{AccountId, SecretText, SignInWay};
pub use account_sheet::ProviderGroup;
pub use action_settings::{
    AddFault, AgentChoice, AgentRow, Agents, ChoiceFault, CloudAccount, CloudAllowed, ModelChoice,
    NewAgent, OwnComputer, PlaceKind, PlaceName, kind_label,
};
pub use base_ids::word::word;
pub use inference_core::{ComputerName, Declined, DeclinedBecause, InferRefusal, ModelRef, Why};
pub use inference_tailnet::{Approval, ComputerCandidate, GuestAnswer, GuestRow, RowState};
pub use keymap::{Action, StandardAction};
pub use memory_core::{HistoryFate, MemoryFate};
/// The value of a settings key as the files hold it.
pub use toml;
