//! All the system seams behind `action_router::Seams`.

use crate::builtin::HostedLink;
use crate::grants::FileGrants;
use crate::infer::{InferdModel, InferdWriter, ReaderClient};
use crate::sheet::SheetConfirmer;
use action_checkpoint_git::GitStore;
use action_review::InferReviewer;
use action_router::{NoLog, Seams};
use agent_session::{Appended, LogError, LogPage, PageSize, Seq, SessionEntry, SessionLog};
use base_clock::SystemClock;
use companion_memory::{MemoryServiceLink, MemorySessionLog};
use companion_memory_core::QueuedSink;
use provenance::SessionId;

/// The sessions' log of the running daemon: memoryd's, or none.
///
/// The daemon reads the log once at start (`Router::adopt_sessions`); a memoryd that cannot serve
/// it (not running, or too old to have `Entries` and `RecordDurable` on the bus) leaves the
/// sessions unrecorded for this run, and the daemon says so, rather than refusing every call for
/// want of a record.
#[derive(Debug)]
pub enum DaemonLog<M: memory_client::Transport> {
    /// Sessions are recorded in memoryd.
    Memory(MemorySessionLog<M, SystemClock>),
    /// Sessions are not recorded.
    Off(NoLog),
}

impl<M: memory_client::Transport> SessionLog for DaemonLog<M> {
    async fn append(
        &self,
        session: &SessionId,
        seq: Seq,
        entry: &SessionEntry,
    ) -> Result<Appended, LogError> {
        match self {
            DaemonLog::Memory(log) => log.append(session, seq, entry).await,
            DaemonLog::Off(log) => log.append(session, seq, entry).await,
        }
    }

    async fn page(
        &self,
        session: &SessionId,
        from: Option<Seq>,
        size: PageSize,
    ) -> Result<LogPage, LogError> {
        match self {
            DaemonLog::Memory(log) => log.page(session, from, size).await,
            DaemonLog::Off(log) => log.page(session, from, size).await,
        }
    }

    async fn sessions(&self) -> Result<Vec<SessionId>, LogError> {
        match self {
            DaemonLog::Memory(log) => log.sessions().await,
            DaemonLog::Off(log) => log.sessions().await,
        }
    }
}

/// The seams of the running daemon.
///
/// Apps and the sheet are over the session bus, the built-in providers in process, models over
/// inferd (`P` is inference-client's transport), memory over memoryd (`M` is memory-client's).
#[derive(Debug)]
pub struct SystemSeams<P: inference_client::Transport, M: memory_client::Transport> {
    /// The apps, and the two built-in providers answered in process.
    pub link: HostedLink<M>,
    /// The sheet.
    pub confirmer: SheetConfirmer,
    /// The reviewer cascade.
    pub reviewer: InferReviewer<InferdModel<P>>,
    /// The consent store.
    pub grants: FileGrants,
    /// The event log's queue.
    pub sink: QueuedSink,
    /// The clock.
    pub clock: SystemClock,
    /// Memory.
    pub memory: MemoryServiceLink<M>,
    /// The policy writer.
    pub writer: InferdWriter<P>,
    /// The reader.
    pub reader: ReaderClient,
    /// The sessions' durable log, in memoryd.
    pub log: DaemonLog<M>,
    /// The restore points of the workspaces sessions work in, kept in their git repositories.
    pub checkpoints: GitStore,
}

impl<P: inference_client::Transport, M: memory_client::Transport> Seams for SystemSeams<P, M> {
    type Link = HostedLink<M>;
    type Confirm = SheetConfirmer;
    type Review = InferReviewer<InferdModel<P>>;
    type Grants = FileGrants;
    type Sink = QueuedSink;
    type Time = SystemClock;
    type Memory = MemoryServiceLink<M>;
    type Writer = InferdWriter<P>;
    type Reading = ReaderClient;
    type Log = DaemonLog<M>;
    type Checkpoints = GitStore;

    fn link(&self) -> &HostedLink<M> {
        &self.link
    }
    fn confirmer(&self) -> &SheetConfirmer {
        &self.confirmer
    }
    fn reviewer(&self) -> &Self::Review {
        &self.reviewer
    }
    fn grants(&self) -> &FileGrants {
        &self.grants
    }
    fn sink(&self) -> &QueuedSink {
        &self.sink
    }
    fn clock(&self) -> &SystemClock {
        &self.clock
    }
    fn memory(&self) -> &Self::Memory {
        &self.memory
    }
    fn writer(&self) -> &Self::Writer {
        &self.writer
    }
    fn reader(&self) -> &ReaderClient {
        &self.reader
    }
    fn log(&self) -> &Self::Log {
        &self.log
    }
    fn checkpoints(&self) -> &GitStore {
        &self.checkpoints
    }
}
