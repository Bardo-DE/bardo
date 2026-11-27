//! Reading the facts of a held process: its cgroup, its image and the ownership of the path to
//! it, and its sandbox's metadata. A handful of small reads, bounded.

use crate::error::ProbeError;
use crate::procfs::{Follow, ProcFs};
use crate::procinfo::parse_flatpak_info;
use crate::walk::dirs_trust;
use crate::{
    CgroupPath, Exe, ExePath, Facts, Pid, Pin, SandboxFacts, Standing, Table, Uid, UnitFacts, judge,
};
use std::io;
use std::path::Path;

/// The most files read for one process. Today three are: cgroup, flatpak metadata, nothing else;
/// the cap is the bound a change must not exceed silently.
pub const PROBE_MAX_READS: usize = 6;
/// The largest file read for one process (the real ones are a few hundred bytes).
pub const PROBE_READ_CAP: u64 = 64 * 1024;

/// A process someone holds.
///
/// It has the number and user it was held as, how firmly, and whether it is still the same live
/// process. The I/O crate implements it for a pidfd; the prober needs nothing more of a process.
pub trait Held {
    /// The process number.
    fn pid(&self) -> Pid;
    /// The user the process runs as.
    fn uid(&self) -> Uid;
    /// How firmly the process was held.
    fn pin(&self) -> Pin;
    /// Whether the held process is still running.
    fn is_alive(&self) -> bool;
}

/// Reads the facts of held processes through a [`ProcFs`].
#[derive(Debug, Clone)]
pub struct Prober<P> {
    fs: P,
}

/// Counts reads against [`PROBE_MAX_READS`].
struct Budget(usize);

impl Budget {
    fn spend(&mut self, what: &'static str) -> Result<(), ProbeError> {
        match self.0.checked_sub(1) {
            Some(left) => {
                self.0 = left;
                Ok(())
            }
            None => Err(ProbeError::Malformed { what }),
        }
    }
}

fn read_error(what: &'static str, source: io::Error) -> ProbeError {
    match source.kind() {
        io::ErrorKind::NotFound => ProbeError::PeerGone,
        io::ErrorKind::InvalidData => ProbeError::Malformed { what },
        _ => ProbeError::Unreadable { what, source },
    }
}

/// A file the process's image sits behind but that this process may not look at, or that is not a
/// path of this mount namespace: the image is hidden, which is a fact, not a failure.
fn is_hiding(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::PermissionDenied | io::ErrorKind::NotFound
    )
}

impl<P: ProcFs> Prober<P> {
    /// A prober reading through `fs`.
    pub fn new(fs: P) -> Prober<P> {
        Prober { fs }
    }

    /// The file-system view.
    pub fn fs(&self) -> &P {
        &self.fs
    }

    /// The facts of `who`, with `unit` as the unit manager's account of its unit (the bus crate
    /// supplies it). No judging. The process is checked still alive at the end: a process that
    /// left while its files were read is [`ProbeError::PeerGone`].
    ///
    /// # Errors
    ///
    /// Returns a [`ProbeError`] when a file of the process cannot be read, the read budget runs
    /// out, or the process left meanwhile.
    pub fn probe(&self, who: &impl Held, unit: Option<UnitFacts>) -> Result<Facts, ProbeError> {
        let mut budget = Budget(PROBE_MAX_READS);
        let dir = self.fs.proc_root().join(who.pid().get().to_string());
        let cgroup = self.cgroup(&mut budget, &dir)?;
        let sandbox = self.sandbox(&mut budget, &dir)?;
        // Inside a sandbox the image path is a path of another mount namespace, so the host's
        // file system cannot say anything about it.
        let exe = match sandbox {
            SandboxFacts::Native => self.exe(&dir)?,
            _ => Exe::Hidden,
        };
        if who.is_alive() {
            Ok(Facts {
                pid: who.pid(),
                uid: who.uid(),
                cgroup,
                exe,
                sandbox,
                unit,
                pin: who.pin(),
            })
        } else {
            Err(ProbeError::PeerGone)
        }
    }

    /// The standing of `who` from its cgroup and image alone (no unit manager): at most `Exe`
    /// proof. A process that cannot be probed is `Unverified`.
    pub fn stand(&self, who: &impl Held, table: &Table) -> Standing {
        match self.probe(who, None) {
            Ok(facts) => judge(&facts, table),
            Err(e) => Standing::unverified(e.why_not()),
        }
    }

    fn text(
        &self,
        budget: &mut Budget,
        path: &Path,
        what: &'static str,
    ) -> Result<String, ProbeError> {
        budget.spend(what)?;
        let bytes = self
            .fs
            .read_capped(path, PROBE_READ_CAP)
            .map_err(|e| read_error(what, e))?;
        String::from_utf8(bytes).map_err(|source| ProbeError::NotText { what, source })
    }

    fn cgroup(&self, budget: &mut Budget, dir: &Path) -> Result<CgroupPath, ProbeError> {
        let what = "the process's cgroup";
        let text = self.text(budget, &dir.join("cgroup"), what)?;
        CgroupPath::from_proc(&text).map_err(|source| ProbeError::BadCgroup { what, source })
    }

    /// A readable `.flatpak-info` is a Flatpak sandbox (one that names nobody is `Malformed`);
    /// none is a native process; one that exists and cannot be read is a failure, not "native".
    fn sandbox(&self, budget: &mut Budget, dir: &Path) -> Result<SandboxFacts, ProbeError> {
        let what = "the sandbox metadata";
        match self.text(budget, &dir.join("root").join(".flatpak-info"), what) {
            Ok(text) => parse_flatpak_info(&text).ok_or(ProbeError::Malformed { what }),
            Err(ProbeError::PeerGone) => Ok(SandboxFacts::Native),
            // Another user's `<pid>/root` is closed to a reader without privilege, exactly as its
            // `exe` is: nothing is known of a sandbox, the image reads as hidden, and only the
            // cgroup can speak (a Flatpak scope there still disagrees with "native").
            Err(ProbeError::Unreadable { source, .. })
                if source.kind() == io::ErrorKind::PermissionDenied =>
            {
                Ok(SandboxFacts::Native)
            }
            Err(other) => Err(other),
        }
    }

    fn exe(&self, dir: &Path) -> Result<Exe, ProbeError> {
        let what = "the process's image";
        let link = dir.join("exe");
        let target = match self.fs.read_link(&link) {
            Ok(target) => target,
            Err(e) if is_hiding(&e) => return Ok(Exe::Hidden),
            Err(e) => return Err(ProbeError::Unreadable { what, source: e }),
        };
        let text = target.to_str().ok_or(ProbeError::Malformed { what })?;
        let clean = text.strip_suffix(" (deleted)").unwrap_or(text);
        let Ok(path) = ExePath::new(clean) else {
            return Ok(Exe::Hidden);
        };
        // The file itself is judged by the open inode; the directories by the path they sit on.
        let file = match self.fs.trust_of(&link, Follow::Yes) {
            Ok(trust) => trust,
            Err(e) if is_hiding(&e) => return Ok(Exe::Hidden),
            Err(e) => return Err(ProbeError::Unreadable { what, source: e }),
        };
        let dirs = match dirs_trust(&self.fs, path.as_path()) {
            Ok(trust) => trust,
            Err(e) if is_hiding(&e) => return Ok(Exe::Hidden),
            Err(e) if e.kind() == io::ErrorKind::InvalidData => {
                return Err(ProbeError::Malformed { what });
            }
            Err(e) => return Err(ProbeError::Unreadable { what, source: e }),
        };
        Ok(Exe::Read {
            path,
            trust: dirs.weaker(file),
        })
    }
}
