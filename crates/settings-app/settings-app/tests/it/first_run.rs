//! `bardo-settings --first-run` run as the binary: once the first run has happened it exits at once, with
//! no window, no bus and nothing written, whatever the environment lacks.

use settings_app_files::{FirstRunMark, StateHome};
use settings_app_testkit::Scratch;
use std::process::{Child, Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_bardo-settings");
const TMP: &str = env!("CARGO_TARGET_TMPDIR");

fn run(scratch: &Scratch) -> std::process::ExitStatus {
    // Nothing of the real session reaches the process: no display, no bus, scratch directories.
    let mut child = Command::new(BIN)
        .arg("--first-run")
        .env_clear()
        .env("HOME", scratch.join("home"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_DATA_DIRS", scratch.join("dirs"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(Started)
        .expect("bardo-settings starts");
    let mut status = None;
    base_testkit::eventually_blocking("--first-run to exit at once", || {
        status = child.0.try_wait().expect("wait");
        status.is_some()
    });
    status.expect("the process exited")
}

/// The process this test started, killed by its PID (and reaped) when the test is done with it,
/// so a hang that fails the test leaves nothing behind.
struct Started(Child);

impl Drop for Started {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn a_second_first_run_exits_at_once_and_touches_nothing() {
    let scratch = Scratch::under(TMP, "first-run-second");
    let state = StateHome(scratch.join("state"));
    let mark = FirstRunMark::under(&state);
    mark.record().expect("the first run happened");
    assert!(run(&scratch).success());
    assert!(mark.done());
    assert!(
        !scratch.join("config").exists(),
        "nothing was written to the config home"
    );
}
