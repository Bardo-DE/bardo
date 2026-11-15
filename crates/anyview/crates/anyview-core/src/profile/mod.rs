//! The one match on `FormatKind`: what each kind of file offers.

mod table;

#[cfg(test)]
mod tests;

pub use table::{actions_for, edits_for, has_stage, mime_for};
