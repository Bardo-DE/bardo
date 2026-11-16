//! The types of other crates that the model's facts are made of, named once here so the window
//! names only `detent-model` and `detent-client`, never almanac, chordkit, docket, porter or
//! toml directly. Each is an id or a fact the model carries as it is; the window words it or
//! hands it back, and holds no rule of its own about it.

pub use memory_core::{HistoryFate, MemoryFate};
pub use keymap::{Action, StandardAction};
pub use action_settings::{
    AddFault, AgentChoice, AgentRow, Agents, ChoiceFault, CloudAccount, CloudAllowed, ModelChoice,
    NewAgent, OwnComputer, PlaceKind, PlaceName, kind_label,
};
pub use account_capability::ModelId;
pub use account_core::{AccountId, SecretText, SignInWay};
pub use account_ids::word::word;
pub use inference_core::{ComputerName, Declined, DeclinedBecause, InferRefusal, ModelRef, Why};
pub use account_sheet::ProviderGroup;
pub use inference_tailnet::{Approval, ComputerCandidate, GuestAnswer, GuestRow, RowState};
/// The value of a settings key as the files hold it.
pub use toml;
