//! The leaf types that computer-use-bus carries inside its bodies and that the capability crates produce.
//!
//! They live here so a consumer that only reads the bus bodies links serde and the desktop
//! vocabulary, not the accessibility, input or capture crates. Each capability crate re-exports
//! its types, so `computer_use_a11y_tree::Role`, `computer_use_seat_input::InputReceipt` and `computer_use_capture::FrameDigest` still
//! name them.
//!
//! Nothing here runs: plain data, no behaviour.

mod digest;
mod role;
mod seat;
mod selector;

pub use digest::FrameDigest;
pub use role::Role;
pub use seat::{InputOrigin, InputReceipt, SeatError, TextPath};
pub use selector::{NameMatch, NodeSelector, SelectorStep};
