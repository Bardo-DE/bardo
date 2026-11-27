//! The zsh and fish completion files: each shell accepts the syntax. They touch the shells
//! installed on this computer, so they are `live`.

use std::path::PathBuf;
use std::process::Command;

fn dist(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../dist/completions")
        .join(name)
}

/// `shell` reads `file` without running it. Fails, saying so, where `shell` is not installed.
fn accepts_the_syntax(shell: &str, file: &str, flag: &str) {
    let out = Command::new(shell)
        .arg(flag)
        .arg(dist(file))
        .output()
        .unwrap_or_else(|e| panic!("{shell} is a required test tool here: {e}"));
    assert!(
        out.status.success(),
        "{shell} rejects {file}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn zsh_accepts_the_syntax() {
    accepts_the_syntax("zsh", "_bardo-do", "-n");
}

#[test]
fn fish_accepts_the_syntax() {
    accepts_the_syntax("fish", "bardo-do.fish", "--no-execute");
}
