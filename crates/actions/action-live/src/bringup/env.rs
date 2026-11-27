//! What a daemon `up` starts is given: the root as its HOME and XDG tree, and a short runtime
//! directory.
//!
//! A root here is the owner's own long-lived directory (`--root`), not a test's scratch one, so it
//! is not a `TestEnv` (which makes and removes its own root). The runtime directory is a link
//! in `/tmp` named after the root's FNV-1a hash and pointing at `<root>/run`: a socket's path is
//! at most 107 bytes, and a root kept under a long path would not fit the sockets under it.

use std::path::{Path, PathBuf};

/// The link's path: `/tmp/dl-<16 hex digits of the root's FNV-1a hash>`.
#[must_use]
pub fn short_run(root: &Path) -> PathBuf {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in root.as_os_str().as_encoded_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    PathBuf::from(format!("/tmp/dl-{hash:016x}"))
}

/// Points the short name at `<root>/run`, replacing a stale link of an earlier `up` of the same
/// root.
///
/// # Errors
///
/// Returns the I/O error of replacing the link.
pub fn link_run(root: &Path) -> std::io::Result<()> {
    let link = short_run(root);
    if link
        .symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
    {
        std::fs::remove_file(&link)?;
    }
    std::os::unix::fs::symlink(root.join("run"), link)
}

/// The variables of a process in `root`: HOME and the XDG directories in it, the short runtime
/// directory, and the `PATH` a user unit gets (an engine's compilers find `ld` on it).
#[must_use]
pub fn env_of(root: &Path) -> Vec<(&'static str, String)> {
    let at = |p: &str| root.join(p).display().to_string();
    vec![
        ("HOME", root.display().to_string()),
        ("PATH", "/usr/bin:/bin".to_owned()),
        ("XDG_DATA_HOME", at("data")),
        ("XDG_DATA_DIRS", at("none")),
        ("XDG_CONFIG_HOME", at("config")),
        ("XDG_CONFIG_DIRS", at("none")),
        ("XDG_CACHE_HOME", at("cache")),
        ("XDG_RUNTIME_DIR", short_run(root).display().to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_short_name_is_short_stable_and_per_root() {
        let long = Path::new("/home/someone/rs-wt/live-eval/tmp/agent-live.bkxE6l/out/root");
        let name = short_run(long);
        assert_eq!(name, short_run(long));
        assert_ne!(name, short_run(Path::new("/elsewhere/root")));
        assert_eq!(
            short_run(Path::new("/tmp/scratch-root")),
            PathBuf::from("/tmp/dl-c037e38128f29d69")
        );
        let socket = name.join("bardo/inference/vllm-qwen3-4b-instruct-2507-fp8.sock");
        assert!(socket.as_os_str().len() < 108, "{}", socket.display());
    }

    #[test]
    fn the_link_reaches_the_roots_run_dir_and_a_second_up_replaces_it() {
        let root = tempfile::tempdir().expect("root");
        std::fs::create_dir(root.path().join("run")).expect("run");
        link_run(root.path()).expect("link");
        std::fs::write(short_run(root.path()).join("probe"), b"x").expect("through the link");
        assert!(root.path().join("run/probe").exists());
        link_run(root.path()).expect("a second link replaces the first");
        std::fs::remove_file(short_run(root.path())).expect("remove");
    }
}
