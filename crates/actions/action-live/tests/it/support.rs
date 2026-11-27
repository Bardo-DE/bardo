//! Shared by the live-harness tests: where the daemon binaries are. They are the prebuilds of the
//! suite (`suites.toml`), found with `accept_kit::bin`: run through `cargo xtask accept agent` or
//! `cargo xtask live session`.
#![allow(
    dead_code,
    reason = "each test binary uses a different part of this shared module"
)]

use action_accept::world::{Binaries, Cassette};

pub fn binaries() -> Binaries {
    // Every test starts here: a test that waits for ever ends the process after 20 minutes.
    base_testkit::hang_guard::arm();
    Binaries::prebuilt()
}

/// Flow (a), the person allows: search, contact search, forward, the closing words.
pub const FLOW_A: Cassette = Cassette(include_str!("../../../dev/accept/cassettes/flow-a.jsonl"));

/// A command line as the words the parser takes.
pub fn words(line: &str) -> Vec<String> {
    line.split_whitespace().map(str::to_owned).collect()
}
