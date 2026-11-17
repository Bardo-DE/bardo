//! `FakeSandbox`: records what it was asked to run and answers from a script. No process, no
//! clock. A test hands the sandbox to the shell tool and keeps a `Seen` handle to look at what
//! reached it.

use crate::bwrap_job::locked;
use crate::output::Tail;
use crate::sandbox::{Captured, ExitReport, Job, KILLED, RunSpec, Sandbox, StartError};
use crate::{CannotSandboxError, SandboxState};
use base_path::AbsPath;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// What the next job does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    /// What it writes.
    pub output: Vec<u8>,
    /// How it ends.
    pub exit: ExitReport,
    /// Whether it has already exited when started (`false`: it runs until killed).
    pub finishes: bool,
}

impl Script {
    /// A job that wrote `output` and exited with `code`.
    #[must_use]
    pub fn done(output: &str, code: u32) -> Self {
        Self {
            output: output.as_bytes().to_vec(),
            exit: ExitReport::Code(code),
            finishes: true,
        }
    }

    /// A job that wrote `output` and keeps running.
    #[must_use]
    pub fn hangs(output: &str) -> Self {
        Self {
            output: output.as_bytes().to_vec(),
            exit: ExitReport::Signal(KILLED.to_owned()),
            finishes: false,
        }
    }
}

/// What the fake has seen.
#[derive(Debug, Default)]
pub struct Log {
    /// Every command it was asked to run, in order.
    pub started: Vec<RunSpec>,
    /// Whether each was killed (by `kill`, or by being dropped), by index into `started`; a job
    /// not killed is still going.
    pub killed: Vec<bool>,
}

/// A shared view of the fake's log.
#[derive(Debug, Clone, Default)]
pub struct Seen(Arc<Mutex<Log>>);

impl Seen {
    /// Every command the sandbox was asked to run.
    #[must_use]
    pub fn started(&self) -> Vec<RunSpec> {
        locked(&self.0).started.clone()
    }

    /// Whether each job was killed.
    #[must_use]
    pub fn killed(&self) -> Vec<bool> {
        locked(&self.0).killed.clone()
    }
}

/// The scripted sandbox.
#[derive(Debug)]
pub struct FakeSandbox {
    available: SandboxState,
    scripts: Mutex<VecDeque<Script>>,
    seen: Seen,
}

impl FakeSandbox {
    /// A sandbox that can run commands, answering from `scripts` in order.
    #[must_use]
    pub fn ready(scripts: Vec<Script>) -> (Self, Seen) {
        Self::with(SandboxState::Ready, scripts)
    }

    /// A sandbox that cannot confine anything, for this reason.
    #[must_use]
    pub fn withheld(why: CannotSandboxError) -> (Self, Seen) {
        Self::with(SandboxState::Cannot(why), Vec::new())
    }

    fn with(available: SandboxState, scripts: Vec<Script>) -> (Self, Seen) {
        let seen = Seen::default();
        let fake = Self {
            available,
            scripts: Mutex::new(scripts.into()),
            seen: seen.clone(),
        };
        (fake, seen)
    }
}

/// A scripted job.
#[derive(Debug)]
pub struct FakeJob {
    tail: Tail,
    script: Script,
    index: usize,
    killed: bool,
    seen: Seen,
}

impl FakeJob {
    fn mark_killed(&mut self) {
        if !self.killed {
            self.killed = true;
            if let Some(killed) = locked(&self.seen.0).killed.get_mut(self.index) {
                *killed = true;
            }
        }
    }
}

impl Job for FakeJob {
    fn output(&mut self) -> Captured {
        self.tail.captured()
    }

    fn exit(&mut self) -> Option<ExitReport> {
        match (self.script.finishes, self.killed) {
            (true, _) => Some(self.script.exit.clone()),
            (false, true) => Some(ExitReport::Signal(KILLED.to_owned())),
            (false, false) => None,
        }
    }

    fn wait(&mut self) -> ExitReport {
        // A job that would run for ever is reported as killed: a test never hangs on the fake.
        self.exit()
            .unwrap_or_else(|| ExitReport::Signal(KILLED.to_owned()))
    }

    fn kill(&mut self) {
        self.mark_killed();
    }
}

impl Drop for FakeJob {
    fn drop(&mut self) {
        self.mark_killed();
    }
}

impl Sandbox for FakeSandbox {
    type Job = FakeJob;

    fn available(&self) -> SandboxState {
        self.available
    }

    fn check(&self, _cwd: &AbsPath) -> SandboxState {
        self.available
    }

    fn start(&self, spec: &RunSpec) -> Result<FakeJob, StartError> {
        if let SandboxState::Cannot(why) = self.available {
            return Err(StartError::Cannot(why));
        }
        let script = locked(&self.scripts)
            .pop_front()
            .unwrap_or_else(|| Script::done("", 0));
        let mut tail = Tail::new(spec.keep);
        tail.push(&script.output);
        let index = {
            let mut log = locked(&self.seen.0);
            log.started.push(spec.clone());
            log.killed.push(false);
            log.started.len() - 1
        };
        Ok(FakeJob {
            tail,
            script,
            index,
            killed: false,
            seen: self.seen.clone(),
        })
    }
}
