//! The completion files under `dist/completions` are thin: each hands what is typed to
//! `bardo-do __complete` and offers what comes back. Each shell checks the
//! syntax and bash runs the function against a stand-in `bardo-do` (`accept::dist`; zsh and fish
//! are `live::dist`). This file reads the files as text.

use std::path::PathBuf;

fn dist(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../dist/completions")
        .join(name)
}

fn read(name: &str) -> String {
    std::fs::read_to_string(dist(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

#[test]
fn the_three_files_ask_bardo_do_what_to_offer() {
    for name in ["bardo-do.bash", "_bardo-do", "bardo-do.fish"] {
        let text = read(name);
        assert!(text.contains("bardo-do __complete"), "{name}");
        assert!(
            !text.contains("--yes"),
            "{name}: nothing here offers a way around the gate"
        );
    }
    assert!(read("_bardo-do").starts_with("#compdef bardo-do"));
}
