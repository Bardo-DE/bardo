//! The integration tests of anyview-image, in one binary so they link once.

#![allow(clippy::unwrap_used)]

mod breadth;
mod decode;
mod edit;
mod encode;
mod faithful_edit;
mod hostile;
mod peek;
mod picture_facts;
mod rotate;
mod support;
