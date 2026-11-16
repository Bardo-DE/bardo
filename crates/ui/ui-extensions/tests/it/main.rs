//! The crate's integration tests: one executable, one module per concept. Every test works in
//! scratch roots standing in for `$XDG_DATA_HOME` and `$XDG_DATA_DIRS`; none reads the real ones.

mod checks;
mod choices;
mod extensions_file;
mod file_icons;
mod scan;
mod support;
mod your_changes;
