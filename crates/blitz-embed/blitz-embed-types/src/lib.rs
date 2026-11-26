//! Plain data that blitz-embed's repairs take and return, and that crates above the renderer name
//! without linking it: the fractional scale (`units`) and the DOM element id (`element_id`).
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

pub mod element_id;
pub mod units;
