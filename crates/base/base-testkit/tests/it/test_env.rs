//! `TestEnv`: what a process it starts sees, and that its directories and link go with it.

use base_testkit::{PrivateBus, TestEnv};
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::process::Stdio;

/// The environment `/usr/bin/env` prints under `env`, as `KEY=value` lines.
fn printed(env: &TestEnv) -> String {
    let out = env
        .command("/usr/bin/env")
        .stdout(Stdio::piped())
        .output()
        .expect("env runs");
    String::from_utf8(out.stdout).expect("utf-8")
}

#[test]
fn a_command_sees_the_scratch_tree_and_nothing_of_ours() {
    let env = TestEnv::new("env-sees");
    let text = printed(&env);
    assert!(text.contains(&format!("HOME={}", env.home().display())));
    assert!(text.contains(&format!("XDG_RUNTIME_DIR={}", env.runtime().display())));
    assert!(text.contains("PATH=/usr/bin:/bin"));
    for line in text.lines() {
        let key = line.split('=').next().unwrap_or_default();
        assert!(
            env.vars().iter().any(|(k, _)| k == key),
            "{key} leaked in from the parent"
        );
    }
}

#[test]
fn the_runtime_dir_is_private_and_an_open_env_is_not() {
    let env = TestEnv::new("env-private");
    let mode = std::fs::metadata(env.root().join("run"))
        .expect("run")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o700);
    let open = TestEnv::new_open("env-open");
    let mode = std::fs::metadata(open.home())
        .expect("home")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o755);
}

#[test]
fn a_long_root_gets_a_short_runtime_link_that_goes_with_the_env() {
    let long = "x".repeat(60);
    let env = TestEnv::new(&long);
    let run = env.runtime().to_owned();
    assert!(
        run.as_os_str().len() <= "/tmp/dl-0123456789abcdef".len(),
        "{}",
        run.display()
    );
    std::fs::write(run.join("probe"), b"x").expect("through the link");
    assert!(env.root().join("run/probe").exists());
    drop(env);
    run.symlink_metadata().unwrap_err();
}

#[test]
fn with_replaces_a_variable_and_buses_are_named_on_request() {
    let session = PrivateBus::start();
    let system = PrivateBus::start_system();
    let env = TestEnv::new("env-bus")
        .with_bus(&session)
        .with_system_bus(&system)
        .with("HOME", "/nowhere");
    let text = printed(&env);
    assert!(text.contains(&format!("DBUS_SESSION_BUS_ADDRESS={}", session.address())));
    assert!(text.contains(&format!("DBUS_SYSTEM_BUS_ADDRESS={}", system.address())));
    assert!(text.contains("HOME=/nowhere"));
}

#[test]
fn spawn_runs_the_program_in_the_env_and_guards_it() {
    let env = TestEnv::new("env-spawn");
    let mut child = env
        .spawn(
            "/bin/sh",
            ["-c", "echo $HOME > \"$HOME/seen\"; exec sleep 300"],
        )
        .expect("spawn");
    base_testkit::eventually_blocking("the program wrote its HOME", || {
        env.home().join("seen").exists()
    });
    let mut seen = String::new();
    std::fs::File::open(env.home().join("seen"))
        .expect("seen")
        .read_to_string(&mut seen)
        .expect("read");
    assert_eq!(seen.trim(), env.home().display().to_string());
    let pid = child.pid();
    assert!(child.child_mut().try_wait().expect("wait").is_none());
    drop(child);
    assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
}
