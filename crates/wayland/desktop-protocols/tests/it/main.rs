//! desktop-protocols' integration tests: the XML, the vocab codes against the generated enums, serde round trips.
//!
//! One integration binary: every former `tests/*.rs` is a module here, and `support` is shared.

mod privilege;
mod round_trip;
mod support;
#[cfg(all(feature = "client", feature = "server"))]
mod vocab_codes;
mod xml_introspection;
