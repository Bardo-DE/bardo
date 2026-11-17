//! The crate's integration tests: one executable, one module per former test file.
//! A separate test target needs a stated reason.

mod accounts_ssr;
mod confirm_ssr;
mod details_lint;
mod helpers_ssr;
#[path = "../../../ui-components/tests/it/support/hygiene.rs"]
mod hygiene;
mod self_lint;
mod specimen_hygiene;
mod stylesheet;
mod tokens;
