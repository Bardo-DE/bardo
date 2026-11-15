//! The crate's integration tests: one executable, one module per former test file.
//! A separate test target needs a stated reason.

#[path = "../../../ds/tests/it/support/hygiene.rs"]
mod hygiene;
mod page_ssr;
