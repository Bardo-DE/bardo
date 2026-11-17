//! The light tier: a cheap look at a file that needs no GPU and no media library.

mod budget;
mod deadline;
mod traits;

#[cfg(test)]
mod tests;

pub use budget::PeekBudget;
pub use deadline::Deadline;
pub use traits::Peek;
