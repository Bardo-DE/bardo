//! For tests: an in-memory pipe, a scripted `Spawn` and a file system in a map. No process, no
//! clock, no network.

use super::files::{FileError, Files};
use super::spawn::{AgentChild, LaunchPlan, SessionMeta, SignIn, Spawn, SpawnError, Spawned};
use agent_client_protocol::{Agent, Client, ConnectTo, Error, Lines};
use base_path::AbsPath;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use futures_util::{SinkExt, StreamExt};
use std::collections::{BTreeMap, VecDeque};
use std::io;
use std::sync::{Arc, Mutex, MutexGuard};

fn locked<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// One end of an in-memory pipe of lines. The agent's end reads and writes lines by hand, as a
/// script does; the client's end is the SDK's transport.
#[derive(Debug)]
pub struct ChannelWire {
    rx: UnboundedReceiver<String>,
    tx: UnboundedSender<String>,
}

/// Two ends: what one writes the other reads.
#[must_use]
pub fn pipe() -> (ChannelWire, ChannelWire) {
    let (a_tx, b_rx) = unbounded();
    let (b_tx, a_rx) = unbounded();
    (
        ChannelWire { rx: a_rx, tx: a_tx },
        ChannelWire { rx: b_rx, tx: b_tx },
    )
}

/// The other end of the pipe is gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the other end of the pipe is gone")]
pub struct PipeClosedError;

impl ChannelWire {
    /// The next line; `None` when the other end is gone. Cancel-safe.
    pub async fn read_line(&mut self) -> Option<String> {
        self.rx.next().await
    }

    /// Writes one line.
    ///
    /// # Errors
    /// Returns [`PipeClosedError`] when the other end is gone.
    #[expect(
        clippy::map_err_ignore,
        reason = "`PipeClosedError` says all there is: the send failed because the other end dropped"
    )]
    pub fn write_line(&mut self, line: String) -> Result<(), PipeClosedError> {
        self.tx.unbounded_send(line).map_err(|_| PipeClosedError)
    }
}

impl ConnectTo<Client> for ChannelWire {
    async fn connect_to(self, client: impl ConnectTo<Agent>) -> Result<(), Error> {
        let sink = self
            .tx
            .sink_map_err(|_| io::Error::from(io::ErrorKind::BrokenPipe));
        let lines = Lines::new(sink, self.rx.map(Ok::<_, io::Error>));
        ConnectTo::<Client>::connect_to(lines, client).await
    }
}

/// What the fake spawner saw.
#[derive(Debug, Default)]
pub struct SpawnLog {
    /// Every plan it was asked to start.
    pub plans: Vec<LaunchPlan>,
    /// How many children were killed.
    pub killed: usize,
    /// How many children were closed (given back what they were lent).
    pub closed: usize,
}

/// A shared view of the spawner's log.
#[derive(Debug, Clone, Default)]
pub struct SpawnSeen(Arc<Mutex<SpawnLog>>);

impl SpawnSeen {
    /// The plans it was asked to start.
    #[must_use]
    pub fn plans(&self) -> Vec<LaunchPlan> {
        locked(&self.0).plans.clone()
    }

    /// Children killed.
    #[must_use]
    pub fn killed(&self) -> usize {
        locked(&self.0).killed
    }

    /// Children closed.
    #[must_use]
    pub fn closed(&self) -> usize {
        locked(&self.0).closed
    }
}

/// A scripted `Spawn`: hands out the pipes it was given, one per start.
#[derive(Debug)]
pub struct FakeSpawn {
    wires: VecDeque<ChannelWire>,
    seen: SpawnSeen,
    meta: Option<SessionMeta>,
    sign_in: Option<SignIn>,
    model: Option<super::models::ModelId>,
}

impl FakeSpawn {
    /// A spawner whose starts return `wires` in order, and a view of what it saw.
    #[must_use]
    pub fn new(wires: Vec<ChannelWire>) -> (Self, SpawnSeen) {
        let seen = SpawnSeen::default();
        (
            Self {
                wires: wires.into(),
                seen: seen.clone(),
                meta: None,
                sign_in: None,
                model: None,
            },
            seen,
        )
    }

    /// The same spawner, handing back `meta` for `session/new` with every start.
    #[must_use]
    pub fn with_meta(mut self, meta: SessionMeta) -> Self {
        self.meta = Some(meta);
        self
    }
}

impl FakeSpawn {
    /// The same spawner, handing back a model to switch to with every start.
    #[must_use]
    pub fn with_model(mut self, model: super::models::ModelId) -> Self {
        self.model = Some(model);
        self
    }

    /// The same spawner, handing back a way to sign in with every start.
    #[must_use]
    pub fn with_sign_in(mut self, sign_in: SignIn) -> Self {
        self.sign_in = Some(sign_in);
        self
    }
}

/// The fake's child.
#[derive(Debug)]
pub struct FakeChild {
    seen: SpawnSeen,
    killed: bool,
    closed: bool,
}

impl AgentChild for FakeChild {
    fn kill(&mut self) {
        if !self.killed {
            self.killed = true;
            locked(&self.seen.0).killed += 1;
        }
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "`AgentChild::close` is async; the fake closes without waiting"
    )]
    async fn close(&mut self) {
        self.kill();
        if !self.closed {
            self.closed = true;
            locked(&self.seen.0).closed += 1;
        }
    }
}

impl Spawn for FakeSpawn {
    type Transport = ChannelWire;
    type Child = FakeChild;

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "`Spawn::spawn` is async; the fake hands out a prepared wire without waiting"
    )]
    async fn spawn(
        &mut self,
        plan: &LaunchPlan,
    ) -> Result<Spawned<ChannelWire, FakeChild>, SpawnError> {
        locked(&self.seen.0).plans.push(plan.clone());
        let wire = self.wires.pop_front().ok_or(SpawnError::Process)?;
        let child = FakeChild {
            seen: self.seen.clone(),
            killed: false,
            closed: false,
        };
        Ok(Spawned {
            transport: wire,
            child,
            meta: self.meta.clone(),
            sign_in: self.sign_in.clone(),
            model: self.model.clone(),
        })
    }
}

#[derive(Debug, Default)]
struct Disk {
    files: BTreeMap<AbsPath, String>,
    links: Vec<(AbsPath, AbsPath)>,
    reads: usize,
    writes: Vec<(AbsPath, String)>,
}

/// A file system in a map, shared between the test and the backend.
#[derive(Debug, Clone, Default)]
pub struct FakeFiles(Arc<Mutex<Disk>>);

impl FakeFiles {
    /// An empty disk.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Puts a file there.
    pub fn put(&self, path: &AbsPath, text: &str) {
        locked(&self.0).files.insert(path.clone(), text.to_owned());
    }

    /// Makes everything at or below `at` really live below `to` (a symbolic link).
    pub fn link(&self, at: &AbsPath, to: &AbsPath) {
        locked(&self.0).links.push((at.clone(), to.clone()));
    }

    /// The text of a file, if there is one.
    #[must_use]
    pub fn text(&self, path: &AbsPath) -> Option<String> {
        locked(&self.0).files.get(path).cloned()
    }

    /// How many reads reached the disk.
    #[must_use]
    pub fn reads(&self) -> usize {
        locked(&self.0).reads
    }

    /// Every write that reached the disk, in order.
    #[must_use]
    pub fn writes(&self) -> Vec<(AbsPath, String)> {
        locked(&self.0).writes.clone()
    }
}

impl Files for FakeFiles {
    fn real(&self, path: &AbsPath) -> Result<AbsPath, FileError> {
        let disk = locked(&self.0);
        for (at, to) in &disk.links {
            if at.covers(path) {
                let rest = &path.as_str()[at.as_str().len()..];
                let joined = format!("{}{rest}", to.as_str());
                return AbsPath::parse(&joined).map_err(|error| {
                    tracing::debug!(%error, "the linked path is not absolute");
                    FileError::Refused
                });
            }
        }
        Ok(path.clone())
    }

    fn read(&mut self, real: &AbsPath, _within: &AbsPath) -> Result<String, FileError> {
        let mut disk = locked(&self.0);
        disk.reads += 1;
        disk.files.get(real).cloned().ok_or(FileError::NotFound)
    }

    fn write(
        &mut self,
        real: &AbsPath,
        _within: &AbsPath,
        content: &str,
    ) -> Result<Option<String>, FileError> {
        let mut disk = locked(&self.0);
        disk.writes.push((real.clone(), content.to_owned()));
        Ok(disk.files.insert(real.clone(), content.to_owned()))
    }

    fn remove(&mut self, real: &AbsPath, _within: &AbsPath) -> Result<(), FileError> {
        locked(&self.0)
            .files
            .remove(real)
            .map(|_| ())
            .ok_or(FileError::NotFound)
    }
}
