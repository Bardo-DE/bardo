//! The bash completion file: bash accepts its syntax and the function hands the typed words to
//! `bardo-do __complete`. (zsh and fish are `live::dist`.)

use std::path::PathBuf;

use base_testkit::TestEnv;

fn dist(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../dist/completions")
        .join(name)
}

#[test]
fn bash_accepts_the_syntax() {
    let env = TestEnv::new("completion-syntax");
    let out = env
        .command("bash")
        .arg("-n")
        .arg(dist("bardo-do.bash"))
        .output()
        .expect("bash runs");
    assert!(
        out.status.success(),
        "bash rejects bardo-do.bash: {}",
        String::from_utf8_lossy(&out.stderr)
    );
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
    let env = TestEnv::new("completion-words");
    let out = env
        .command("bash")
        .arg("-c")
        .arg(script)
        .output()
        .expect("bash runs");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "__complete|mail|thread.archive|--|"
    );
}
