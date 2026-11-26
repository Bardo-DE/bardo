//! Every app we ship can be driven from a command line (cli.md section 6): each app under
//! `crates/` that has a `dist/intents` directory passes the same conformance check
//! `agent-eval --check-app` runs.

use action_eval::{Level, check_app};
use std::path::{Path, PathBuf};

/// The workspace root: the nearest parent whose `Cargo.toml` declares `[workspace]`.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|dir| {
            std::fs::read_to_string(dir.join("Cargo.toml"))
                .is_ok_and(|text| text.lines().any(|l| l.trim() == "[workspace]"))
        })
        .expect("a workspace root above action-eval")
        .to_owned()
}

/// The app directories: each `crates/<app>` that has a `dist/intents` directory.
fn apps() -> Vec<PathBuf> {
    let crates = workspace_root().join("crates");
    let mut apps: Vec<PathBuf> = std::fs::read_dir(&crates)
        .expect("crates directory")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.join("dist/intents").is_dir())
        .collect();
    apps.sort();
    apps
}

#[test]
fn every_shipped_app_passes_the_conformance_check() {
    let apps = apps();
    assert!(!apps.is_empty(), "no app with dist/intents found");
    let mut failures = Vec::new();
    for app in &apps {
        let report = check_app(app).unwrap_or_else(|e| panic!("{}: {e:?}", app.display()));
        failures.extend(
            report
                .findings
                .into_iter()
                .filter(|f| f.level == Level::Fail)
                .map(|f| format!("{}: {}: {}", app.display(), f.file.display(), f.what)),
        );
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
