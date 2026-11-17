//! What the composition root is injected with (A1, A4, A5): the file, bus and process seams.
//!
//! `State::new` builds the real ones from the start configuration ([`Seams::real`]); the test
//! kit's `Fixture` builds the fakes and calls `State::with_seams`. Nothing in the loop opens a
//! file, calls a bus or starts a process any other way.

use crate::a11y::{A11yChecks, NoA11y};
use crate::endpoint::Endpoint;
use crate::lock_marker::{FileLockMarker, LockMarker, NoLockMarker};
use crate::notify::{Notify, SocketNotify};
use crate::privacy::{
    BackgroundPrivacyStore, FloorOnlyStore, NoPrivacyStore, PrivacyFiles, PrivacyStore, SystemDir,
};
use crate::space_store::{BackgroundSpaceStore, EphemeralSpaceStore, SpaceStore, StateDirs};
use crate::space_sync::SpaceSync;
use crate::spawn::{ChildEnv, ProcessSpawner, Spawner};
use base_time::{Clock, FixedClock, SystemClock, UnixSeconds};
use compositor_backend::tty::TtySeams;
use compositor_ipc::shell_calls::{ShellCalls, ZbusShellCalls};
use std::fmt;
use std::sync::Arc;

/// The injected seams.
pub struct Seams {
    /// Where the Spaces file is stored.
    pub space_store: Box<dyn SpaceStore>,
    /// How a binding calls the shell.
    pub shell_calls: Box<dyn ShellCalls>,
    /// How a program is started.
    pub spawner: Box<dyn Spawner>,
    /// The variables every program a binding starts gets (the Wayland socket's name, known once
    /// `run` has bound it).
    pub child_env: ChildEnv,
    /// The Spaces registered with the account service (none until `run` is given a bus).
    pub space_sync: SpaceSync,
    /// How the service manager is told the compositor is ready (nothing until `run` is given a socket).
    pub notify: Box<dyn Notify>,
    /// The tty backend's seams, when they are injected (a test); none builds the real ones.
    pub tty: Option<TtySeams>,
    /// The session lock's marker (none kept until a runtime directory is given).
    pub lock_marker: Box<dyn LockMarker>,
    /// The person's privacy book and the administrator's floor (nothing kept until a state
    /// directory is given).
    pub privacy: Box<dyn PrivacyStore>,
    /// The date a kept answer is stamped with (the epoch until the run gives it the system's).
    pub wall: Arc<dyn Clock>,
    /// Who checks that a window's accessibility root belongs to the window's own program
    /// (nobody until `run` is given a bus: a root stays unknown).
    pub a11y: Box<dyn A11yChecks>,
}

impl fmt::Debug for Seams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Seams")
            .field("space_store", &self.space_store)
            .field("shell_calls", &self.shell_calls)
            .field("spawner", &self.spawner)
            .field("child_env", &self.child_env)
            .field("space_sync", &self.space_sync)
            .field("notify", &self.notify)
            .field("tty", &self.tty)
            .field("lock_marker", &self.lock_marker)
            .field("privacy", &self.privacy)
            .field("wall", &self.wall.now())
            .field("a11y", &self.a11y)
            .finish()
    }
}

impl Seams {
    /// These seams.
    #[must_use]
    pub fn new(
        space_store: Box<dyn SpaceStore>,
        shell_calls: Box<dyn ShellCalls>,
        spawner: Box<dyn Spawner>,
    ) -> Seams {
        Seams {
            space_store,
            shell_calls,
            spawner,
            child_env: ChildEnv::default(),
            space_sync: SpaceSync::none(),
            notify: Box::new(SocketNotify::new(Endpoint::Unset)),
            tty: None,
            lock_marker: Box::new(NoLockMarker),
            privacy: Box::new(NoPrivacyStore),
            wall: Arc::new(FixedClock::new(UnixSeconds(0))),
            a11y: Box::new(NoA11y),
        }
    }

    /// The same with the privacy book and floor kept in `privacy`.
    #[must_use]
    pub fn with_privacy(self, privacy: Box<dyn PrivacyStore>) -> Seams {
        Seams { privacy, ..self }
    }

    /// The same with kept answers stamped from `wall`.
    #[must_use]
    pub fn with_wall(self, wall: Arc<dyn Clock>) -> Seams {
        Seams { wall, ..self }
    }

    /// The same with accessibility roots checked by `a11y`.
    #[must_use]
    pub fn with_a11y(self, a11y: Box<dyn A11yChecks>) -> Seams {
        Seams { a11y, ..self }
    }

    /// The same with the session lock's marker kept in `lock_marker`.
    #[must_use]
    pub fn with_lock_marker(self, lock_marker: Box<dyn LockMarker>) -> Seams {
        Seams {
            lock_marker,
            ..self
        }
    }

    /// The same with Spaces registered through `space_sync`.
    #[must_use]
    pub fn with_space_sync(self, space_sync: SpaceSync) -> Seams {
        Seams { space_sync, ..self }
    }

    /// The same with the service manager told through `notify`.
    #[must_use]
    pub fn with_notify(self, notify: Box<dyn Notify>) -> Seams {
        Seams { notify, ..self }
    }

    /// The same with the tty backend's seams injected (the `FakeKms` suite).
    #[must_use]
    pub fn with_tty(self, tty: TtySeams) -> Seams {
        Seams {
            tty: Some(tty),
            ..self
        }
    }

    /// The real seams: the Spaces file and the privacy book in `home` (none kept for an
    /// ephemeral run), the administrator's floor in `system` (read by every run, ephemeral or
    /// not), the system clock, the session bus on first use, child processes.
    #[must_use]
    pub fn real(home: Option<&StateDirs>, system: &SystemDir) -> Seams {
        let space_store: Box<dyn SpaceStore> = match home {
            Some(dirs) => Box::new(BackgroundSpaceStore::new(dirs)),
            None => Box::new(EphemeralSpaceStore),
        };
        let lock_marker: Box<dyn LockMarker> = match home {
            Some(dirs) => match dirs.runtime_dir() {
                Some(dir) => Box::new(FileLockMarker::in_dir(dir)),
                None => Box::new(NoLockMarker),
            },
            None => Box::new(NoLockMarker),
        };
        let privacy: Box<dyn PrivacyStore> = match home {
            Some(dirs) => Box::new(BackgroundPrivacyStore::new(PrivacyFiles {
                book: dirs.privacy_file(),
                floor: crate::privacy::floor_in(system),
            })),
            None => Box::new(FloorOnlyStore::new(crate::privacy::floor_in(system))),
        };
        Seams::new(
            space_store,
            Box::new(ZbusShellCalls::lazy()),
            Box::new(ProcessSpawner::default()),
        )
        .with_lock_marker(lock_marker)
        .with_privacy(privacy)
        .with_wall(Arc::new(SystemClock))
    }
}
