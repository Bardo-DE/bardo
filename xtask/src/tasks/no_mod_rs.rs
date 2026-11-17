//! `cargo xtask check no-mod-rs`: a module `foo` with children is `foo.rs` beside a `foo/`
//! directory, never `foo/mod.rs`.

use anyhow::{Context, Result};

use super::{finish, git_lines};
use crate::workspace::Workspace;

/// Whether the tracked file `path` is a `mod.rs`.
fn is_mod_rs(path: &str) -> bool {
    path == "mod.rs" || path.ends_with("/mod.rs")
}

/// Checks every tracked file under `crates/` and `xtask/`.
pub fn run() -> Result<bool> {
    let ws = Workspace::load()?;
    let files =
        git_lines(&ws.root, &["ls-files", "crates", "xtask"])?.context("git ls-files failed")?;
    let mut green = true;
    for file in files.iter().filter(|file| is_mod_rs(file)) {
        eprintln!("no-mod-rs: {file}: name it after its directory (`foo/mod.rs` is `foo.rs`)");
        green = false;
    }
    Ok(finish("check no-mod-rs", green))
}

#[cfg(test)]
mod tests {
    use super::is_mod_rs;

    #[test]
    fn a_mod_rs_file_is_a_hit() {
        assert!(is_mod_rs("crates/a/b/src/foo/mod.rs"));
        assert!(is_mod_rs("mod.rs"));
        assert!(!is_mod_rs("crates/a/b/src/foo.rs"));
        assert!(!is_mod_rs("crates/a/b/src/foo/module.rs"));
        assert!(!is_mod_rs("crates/a/b/src/foo/not_mod.rs"));
    }
}
