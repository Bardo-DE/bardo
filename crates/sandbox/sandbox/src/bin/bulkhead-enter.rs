//! `bulkhead-enter [--read P]... [--write P]... [--device P]... NETWORK -- <program> [args...]`
//!
//! See [`sandbox::enter::enter_main`].

fn main() -> std::process::ExitCode {
    sandbox::enter::enter_main("bulkhead-enter")
}
