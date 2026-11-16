//! The dock's pieces, drawn for the shell to lay out.
//!
//! They are the tile with its badge, progress bar and running dot, the label above it
//! and the optional reflective floor. The shell lays the tiles out and runs their magnification
//! and bounce; these draw what one tile is (design/30 section 2.10, design/10-BEHAVIOUR-dock.md).

pub(crate) mod parts;
pub mod tile;
