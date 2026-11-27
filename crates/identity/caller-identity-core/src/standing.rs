//! The result of judging a process: who it is, as far as it can be shown.

use crate::facts::{Engine, Pin};
use crate::ids::{AppKey, ExePath, RoleName};

/// How much was shown about a service.
///
/// Ordered by strength: `Full` includes everything below it, and `Unit` ranks above `Exe` because
/// the manager's word on a unit (its definition, its loader variables, a root-only program) shows
/// more than an image read alone, though `Unit` lacks the image read itself. The ladder has one more step at the bottom,
/// `Dev`, which only a developer row ([`Table::with_dev_rows`](crate::Table::with_dev_rows)) gives; a
/// table without such rows never yields it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Proof {
    /// The image matched a developer row, a row from a developer's own file. The image may be
    /// one the person wrote, so this shows nothing about the program; it is the lowest proof,
    /// below [`Proof::Cgroup`], and no requirement of `Cgroup`, `Exe` or `Full` is met by it. A
    /// consumer accepts it only by asking for it ([`crate::Requirement::dev`]), and only in its
    /// own debug builds. The variant is always present because the judgement grants it to a
    /// developer row; only `caller-identity-dev`, never part of a shipped program, makes such rows.
    Dev,
    /// Pinned, uid known, the cgroup names the unit. A same-user process can move itself into a
    /// delegated cgroup, so this is a speed bump, never a boundary. It is all that can be had
    /// when the image cannot be read.
    Cgroup,
    /// The image was read, it is root-owned all the way up and the row's `exe` patterns match it.
    Exe,
    /// The image is hidden from the prober, and the manager says all that `Full` needs of the
    /// unit: the process is the main process of the row's unit, from a root-owned fragment with
    /// no user-writable drop-in, not transient, unsetting every loader variable, whose
    /// `ExecStart` program is one of the row's and root-only all the way up. This is what a
    /// sandboxed user service can have: in its user namespace the kernel hides every other
    /// process's `exe` and `environ`, so the running image itself cannot be read.
    Unit,
    /// `Exe`, and the unit is real: the manager says it is this process's unit, from a root-owned
    /// fragment with no user-writable drop-in, not transient, unsetting every loader variable, and running this program.
    Full,
}

/// Why nothing can be remembered about a process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WhyNot {
    /// The process was gone before its facts were read.
    PeerGone,
    /// The image, or a directory above it, can be written by someone other than root.
    ImageWritable,
    /// The image could not be read and no row names the cgroup.
    ImageHidden,
    /// A sandbox's metadata and its cgroup disagree, or the metadata is unusable.
    SandboxDisagrees,
    /// The process's facts could not be read.
    Unreadable,
}

/// Who a process is.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Standing {
    /// A table row matched on every check its proof level needs.
    Service {
        /// The row's app.
        app: AppKey,
        /// The row's role.
        role: RoleName,
        /// What was shown.
        proof: Proof,
        /// How firmly the process was held.
        pin: Pin,
    },
    /// A sandbox attested by the kernel or the runtime (never by a name).
    Sandboxed {
        /// The sandbox's application.
        app: AppKey,
        /// The sandbox engine.
        engine: Engine,
    },
    /// A root-owned image that is in no row. `scope_app` is the launcher's scope name: a claim.
    Program {
        /// The image.
        exe: ExePath,
        /// The app the launcher's scope name claims, for display and grouping only.
        scope_app: Option<AppKey>,
    },
    /// Nothing that can be remembered: a user-writable image, a hidden image, a peer that left.
    Unverified {
        /// The image's file name, when it was read.
        exe_name: Option<String>,
        /// The app the launcher's scope names (`app-<id>-<n>.scope`), carried only when the image
        /// is user-writable (`WhyNot::ImageWritable`): a claim for display and for a consumer that
        /// accepted scope names before (the accounts service's accounts), never a role, never a service, and
        /// nothing about it may be remembered by its image. `None` for every other reason.
        scope_app: Option<AppKey>,
        /// Why nothing is known.
        why: WhyNot,
    },
}

impl Standing {
    /// Nothing is known about the process beyond `why`.
    #[must_use]
    pub fn unverified(why: WhyNot) -> Standing {
        Standing::Unverified {
            exe_name: None,
            scope_app: None,
            why,
        }
    }
}
