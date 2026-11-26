//! The base layer of the design system: vocabulary, geometry, time, errors, colour and encodings,
//! plain data and maths with no renderer and no Dioxus, so a crate that only reads settings or
//! services can use it.
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

pub mod base64;
pub mod colour;
pub mod command;
pub mod error;
pub mod geometry;
pub mod machine;
pub mod palette;
pub mod png;
pub mod press;
pub mod selection;
pub mod standard_action;
pub mod text;
pub mod time;
pub mod vocab;

#[cfg(any(test, feature = "testing"))]
pub mod testing;
