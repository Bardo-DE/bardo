//! `bardo-sandbox-enter [--read P]... [--write P]... [--device P]... NETWORK -- <program> [args...]`
//!
//! See [`sandbox_enter::enter_main`].

fn main() -> std::process::ExitCode {
    sandbox_enter::enter_main("bardo-sandbox-enter")
}
