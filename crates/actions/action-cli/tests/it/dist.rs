//! The completion files under `dist/completions` are thin: each hands what is typed to
//! `bardo-do __complete` and offers what comes back. Each shell checks the
//! syntax (zsh and fish only where installed: their tests are `#[ignore]`d); bash also runs the
//! function against a stand-in `bardo-do`.

use std::path::PathBuf;
use std::process::Command;

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
fn bash_accepts_the_syntax() {
    accepts_the_syntax("bash", "bardo-do.bash", "-n");
}

#[test]
#[ignore = "needs zsh installed"]
fn zsh_accepts_the_syntax() {
    accepts_the_syntax("zsh", "_bardo-do", "-n");
}

#[test]
#[ignore = "needs fish installed"]
fn fish_accepts_the_syntax() {
    accepts_the_syntax("fish", "bardo-do.fish", "--no-execute");
}

#[test]
fn bash_hands_the_typed_words_to_bardo_do() {
    let script = format!(
        r#"
bardo-do() {{ printf '%s\n' "$@"; }}
source {}
COMP_WORDS=(bardo-do mail thread.archive --)
COMP_CWORD=3
_bardo_do
printf '%s|' "${{COMPREPLY[@]}}"
"#,
        dist("bardo-do.bash").display()
    );
    let out = Command::new("bash")
        .arg("-c")
        .arg(script)
        .output()
        .expect("bash runs");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "__complete|mail|thread.archive|--|"
    );
}
