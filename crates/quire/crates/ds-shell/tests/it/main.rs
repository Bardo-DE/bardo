//! The crate's integration tests: one executable, one module per former test file.
//! A separate test target needs a stated reason.

mod accounts_ssr;
mod confirm_ssr;
mod details_lint;
#[path = "../../../ds/tests/it/support/golden.rs"]
mod golden;
mod golden_hygiene;
mod helpers_ssr;
#[path = "../../../ds/tests/it/support/hygiene.rs"]
mod hygiene;
mod self_lint;
mod stylesheet;
mod tokens;
