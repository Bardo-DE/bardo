//! `MenuItem`: what a menu lists, as data, and how it draws.

pub(crate) mod context;
#[expect(clippy::module_inception, reason = "the layout names the file for its one concept")]
pub mod item;
pub(crate) mod lines;
pub(crate) mod swatches;
pub mod text;
pub(crate) mod tip;
pub(crate) mod view;
