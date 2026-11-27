//! `bardo-settings --first-run` run as the binary: once the first run has happened it exits at once, with
//! no window, no bus and nothing written, whatever the environment lacks.

use base_testkit::TestEnv;
use settings_app_files::{FirstRunMark, StateHome};
use std::process::Stdio;

const BIN: &str = env!("CARGO_BIN_EXE_bardo-settings");

fn run(env: &TestEnv) -> std::process::ExitStatus {
    // Nothing of the real session reaches the process: no display, a bus that points at nothing,
    // scratch directories. Guarded, so a hang that fails the test leaves nothing behind.
    let mut command = env.command(BIN);
    command
        .arg("--first-run")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = base_testkit::Reaped::spawn(&mut command).expect("bardo-settings starts");
    let mut status = None;
    base_testkit::eventually_blocking("--first-run to exit at once", || {
        status = child.child_mut().try_wait().expect("wait");
        status.is_some()
    });
    status.expect("the process exited")
}

#[test]
fn a_second_first_run_exits_at_once_and_touches_nothing() {
    let scratch = TestEnv::new("first-run-second");
    let state = StateHome(scratch.state());
    let mark = FirstRunMark::under(&state);
    mark.record().expect("the first run happened");
    assert!(run(&scratch).success());
    assert!(mark.done());
    assert!(
        std::fs::read_dir(scratch.config())
            .expect("config home")
            .next()
            .is_none(),
        "nothing was written to the config home"
    );
}
