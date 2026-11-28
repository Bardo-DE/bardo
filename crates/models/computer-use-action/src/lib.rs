//! The provider-neutral computer-use action vocabulary.
//!
//! Every point carries the coordinate space it lives in, so mixing a window point with an image
//! point does not compile. Models, parsers and the run machine all speak `ComputerUseAction<S>`; only
//! the space parameter changes as a point travels from the model's grid to the leased window.
//!
//! ```
//! use computer_use_action::{ActionClass, Button, ClickCount, ComputerUseAction, Target};
//! use computer_use_geometry::{Coord, Point, WindowSpace};
//! use std::collections::BTreeSet;
//!
//! let at = Target::Point(Point::<WindowSpace>::new(Coord(10), Coord(20)));
//! let click = ComputerUseAction::Click { at, button: Button::Left, count: ClickCount::One, mods: BTreeSet::new() };
//! assert_eq!(click.class(), ActionClass::Pointer);
//!
//! let json = serde_json::to_string(&click)?;
//! let back: ComputerUseAction<WindowSpace> = serde_json::from_str(&json)?;
//! assert_eq!(back, click);
//! # Ok::<(), serde_json::Error>(())
//! ```
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

mod action;
mod gate;
mod keys;
mod target;
mod text;

pub use action::{ActionClass, ComputerUseAction, Extracted, FinishOutcome, ScrollBy};
pub use gate::{RunMode, WindowClass, WindowTrust};
pub use keys::{
    Button, Chord, ChordError, ClickCount, Notches, Repeat, RepeatError, ScrollDir, WaitError,
    WaitMs,
};
pub use target::{NodeId, Target};
pub use text::{Choice, ExtractedText, FieldName, Summary, TextError, TypedText};
