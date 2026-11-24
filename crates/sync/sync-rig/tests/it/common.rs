//! What the rig's integration tests share: a private bus, a scratch directory, a daemon process
//! that is stopped by its PID. Nothing here reaches the real session, `~/.config` or `/etc`.
#![allow(
    dead_code,
    reason = "a helper shared by the test modules; each module uses some of them"
)]


use std::path::Path;
use std::process::{Child, Command, Stdio};

/// A child process, killed by its PID when the guard goes.
#[derive(Debug)]
pub struct Proc {
    pub child: Child,
}

impl Proc {
    /// Spawns `program` with a clean environment plus `envs`.
    pub fn spawn(program: &str, args: &[&str], envs: &[(&str, &Path)]) -> Self {
        let mut command = Command::new(program);
        command
            .env_clear()
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        for (name, value) in envs {
            command.env(name, value);
        }
        Self {
            child: command.spawn().expect("the rig binary starts"),
        }
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Sends SIGTERM to this PID and waits for the process to end; its exit status.
    pub fn terminate(&mut self) -> std::process::ExitStatus {
        let pid = rustix::process::Pid::from_raw(i32::try_from(self.child.id()).expect("pid"))
            .expect("a pid");
        rustix::process::kill_process(pid, rustix::process::Signal::TERM).expect("SIGTERM");
        self.child.wait().expect("the process ends")
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub use base_testkit::eventually_some as eventually;
