//! Scratch repositories: a temporary folder with its own HOME and git configuration, so the
//! person's real configuration, repositories and index are never read or written.

use action_checkpoint::WorkRoot;
use action_checkpoint_git::GitStore;
use base_path::AbsPath;
use base_testkit::TestEnv;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Scratch {
    env: TestEnv,
}

fn git_exists() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|out| out.status.success())
}

impl Scratch {
    /// A scratch folder. Fails, saying so, where git is missing: it is a required test tool.
    pub fn new() -> Scratch {
        assert!(git_exists(), "git is a required test tool (install git)");
        let env = TestEnv::new("git");
        let root = env.root().to_string_lossy().into_owned();
        // Nothing of the person's configuration: no system or global file, and no repository
        // above the scratch folder.
        let env = env
            .with("GIT_CONFIG_NOSYSTEM", "1")
            .with("GIT_CONFIG_GLOBAL", "/dev/null")
            .with("GIT_CEILING_DIRECTORIES", &root);
        Scratch { env }
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.env.root().join(name)
    }

    /// The environment git runs in: nothing of the person's configuration.
    pub fn env(&self) -> Vec<(String, String)> {
        self.env.vars()
    }

    /// Runs git by hand in `cwd`, as a person would; what it printed.
    pub fn git(&self, cwd: &Path, args: &[&str]) -> String {
        let mut command = self.env.command("git");
        command.args(args).current_dir(cwd);
        command
            .env("GIT_AUTHOR_NAME", "Tester")
            .env("GIT_AUTHOR_EMAIL", "tester@example.com")
            .env("GIT_COMMITTER_NAME", "Tester")
            .env("GIT_COMMITTER_EMAIL", "tester@example.com");
        let out = command.output().expect("git runs");
        assert!(out.status.success(), "git {args:?} failed");
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    /// Runs git by hand against the store's own repository (`shadow.git` under the state folder),
    /// where the restore points are; what it printed.
    pub fn points(&self, args: &[&str]) -> String {
        let state = self.path("state");
        let shadow = state.join("shadow.git").to_string_lossy().into_owned();
        let mut all = vec!["--git-dir", shadow.as_str()];
        all.extend_from_slice(args);
        self.git(&state, &all)
    }

    /// A repository with no commits whose `*.log` files are ignored.
    pub fn repo(&self, name: &str) -> PathBuf {
        let repo = self.path(name);
        std::fs::create_dir_all(&repo).expect("repo folder");
        self.git(&repo, &["init", "-q"]);
        std::fs::write(repo.join(".git/info/exclude"), "*.log\n").expect("exclude");
        repo
    }

    /// The store under test, keeping its private indexes in the scratch folder.
    pub fn store(&self) -> GitStore {
        let state = self.path("state");
        GitStore::new(abs(&state)).with_env(self.env())
    }
}

pub fn abs(path: &Path) -> AbsPath {
    AbsPath::parse(&path.to_string_lossy()).expect("an absolute path")
}

pub fn root(path: &Path) -> WorkRoot {
    WorkRoot::new(abs(path))
}
