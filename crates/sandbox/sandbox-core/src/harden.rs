//! The hardening a caller chooses, as typed values with safe defaults.
//!
//! Nothing here runs anything. `Limits` says how much a sandbox may use, `LimitPolicy` says how
//! (or whether) that is enforced, and `Hardening` gathers the choices that `BwrapSandbox` turns
//! into a command line. A person who accepts no limits has to say so with `AcceptNoLimits`; the
//! default is the strict choice, and a sandbox that cannot apply it refuses to start.

use base_path::AbsPath;
use serde::{Deserialize, Serialize};

/// A size in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Bytes(pub u64);

impl Bytes {
    /// `n` mebibytes.
    #[must_use]
    pub const fn mib(n: u64) -> Self {
        Self(n * 1024 * 1024)
    }

    /// `n` gibibytes.
    #[must_use]
    pub const fn gib(n: u64) -> Self {
        Self(n * 1024 * 1024 * 1024)
    }
}

/// The page size limits are rounded to: the kernel rounds cgroup memory limits down to it, so a
/// limit that is not a multiple would read back differently from what was set.
const PAGE: u64 = 4096;

/// Why a set of limits cannot be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum LimitsError {
    /// Memory is not a whole number of pages, or is less than one page.
    #[error("the memory limit must be a whole number of 4096-byte pages, at least one")]
    Memory,
    /// No process may run.
    #[error("the process limit must be above zero")]
    Pids,
    /// No CPU time is allowed.
    #[error("the CPU-seconds limit must be above zero")]
    CpuSeconds,
    /// The scratch directories would have no room.
    #[error("the scratch directory size must be above zero")]
    Tmpfs,
}

/// What one sandbox may use. Every value is checked when the limits are made (`new`, or reading
/// them from a file), so a `Limits` that exists is one a sandbox can apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawLimits")]
pub struct Limits {
    memory: Bytes,
    pids: u32,
    cpu_seconds: u32,
    tmpfs: Bytes,
}

/// The limits as a file or a caller writes them, before they are checked. Any missing field is the
/// default's.
#[derive(Deserialize)]
#[serde(default)]
struct RawLimits {
    memory: Bytes,
    pids: u32,
    cpu_seconds: u32,
    tmpfs: Bytes,
}

impl Default for RawLimits {
    fn default() -> Self {
        let defaults = Limits::default();
        Self {
            memory: defaults.memory,
            pids: defaults.pids,
            cpu_seconds: defaults.cpu_seconds,
            tmpfs: defaults.tmpfs,
        }
    }
}

impl TryFrom<RawLimits> for Limits {
    type Error = LimitsError;

    fn try_from(raw: RawLimits) -> Result<Self, LimitsError> {
        Self::new(raw.memory, raw.pids, raw.cpu_seconds, raw.tmpfs)
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            memory: Bytes::gib(2),
            pids: 256,
            cpu_seconds: 900,
            tmpfs: Bytes::mib(256),
        }
    }
}

impl Limits {
    /// Limits of `memory` for everything in the sandbox together (swap is not allowed on top),
    /// `pids` processes and threads together, `cpu_seconds` of CPU time for one process before the
    /// kernel stops it, and `tmpfs` for each in-memory scratch directory (`/tmp`, `/var/tmp`,
    /// `/run`, ...).
    ///
    /// # Errors
    /// Returns the first value that is not usable: a zero, or a memory that is not a whole number
    /// of pages.
    pub fn new(
        memory: Bytes,
        pids: u32,
        cpu_seconds: u32,
        tmpfs: Bytes,
    ) -> Result<Self, LimitsError> {
        if memory.0 < PAGE || !memory.0.is_multiple_of(PAGE) {
            return Err(LimitsError::Memory);
        }
        if pids == 0 {
            return Err(LimitsError::Pids);
        }
        if cpu_seconds == 0 {
            return Err(LimitsError::CpuSeconds);
        }
        if tmpfs.0 == 0 {
            return Err(LimitsError::Tmpfs);
        }
        Ok(Self {
            memory,
            pids,
            cpu_seconds,
            tmpfs,
        })
    }

    /// Memory for everything in the sandbox together.
    #[must_use]
    pub fn memory(&self) -> Bytes {
        self.memory
    }

    /// Processes and threads in the sandbox together.
    #[must_use]
    pub fn pids(&self) -> u32 {
        self.pids
    }

    /// CPU seconds one process may use before the kernel stops it.
    #[must_use]
    pub fn cpu_seconds(&self) -> u32 {
        self.cpu_seconds
    }

    /// The size of each in-memory scratch directory.
    #[must_use]
    pub fn tmpfs(&self) -> Bytes {
        self.tmpfs
    }
}

/// The explicit statement that the person accepts a sandbox with no resource limits. Without
/// it, a sandbox that cannot apply limits does not start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptNoLimits;

/// How limits are enforced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitPolicy {
    /// In a transient systemd user scope (cgroup): memory, pids and CPU seconds, for the whole
    /// sandbox. The default. Refuses to start where systemd cannot apply them.
    Cgroup(Limits),
    /// Per-process kernel limits only (`ulimit`): CPU seconds, and address space for the
    /// memory. There is no cap on the number of processes. For hosts without a user manager;
    /// weaker, so it is chosen, never a silent fallback.
    Rlimit(Limits),
    /// No resource limits, accepted by the person.
    Unlimited(AcceptNoLimits),
}

impl Default for LimitPolicy {
    fn default() -> Self {
        LimitPolicy::Cgroup(Limits::default())
    }
}

impl LimitPolicy {
    /// The limits, where the policy has any.
    #[must_use]
    pub fn limits(&self) -> Option<&Limits> {
        match self {
            LimitPolicy::Cgroup(l) | LimitPolicy::Rlimit(l) => Some(l),
            LimitPolicy::Unlimited(_) => None,
        }
    }
}

/// Everything a caller chooses about hardening.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Hardening {
    /// Resource limits.
    pub limits: LimitPolicy,
    /// Whether the default syscall filter is installed (no nested user namespaces, `ptrace`,
    /// `bpf`, key rings, mounts, `io_uring` and similar). A sandbox that cannot install it is
    /// not created. Off only when the person chose so.
    pub seccomp: bool,
    /// Whether `/opt` is visible (read-only).
    pub opt: bool,
    /// Extra host paths made visible read-only at the same path: the escape hatch for a
    /// toolchain that lives outside `/usr` (`/nix`, a version manager under a home directory).
    pub extra_ro: Vec<AbsPath>,
    /// Whether the sandbox starts although a descriptor above 2 would be inherited, chosen by
    /// the person; otherwise it refuses to start (`StartError::InheritedFds`).
    pub inherited_fds: bool,
}

impl Default for Hardening {
    fn default() -> Self {
        Self {
            limits: LimitPolicy::default(),
            seccomp: true,
            opt: false,
            extra_ro: Vec::new(),
            inherited_fds: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each unusable number is refused where the limits are made, by name.
    #[test]
    fn limits_that_are_not_usable_are_refused_when_made() {
        let (m, t) = (Bytes::gib(1), Bytes::mib(8));
        let cases = [
            (Limits::new(Bytes(0), 1, 1, t), LimitsError::Memory),
            (Limits::new(Bytes(4097), 1, 1, t), LimitsError::Memory),
            (Limits::new(m, 0, 1, t), LimitsError::Pids),
            (Limits::new(m, 1, 0, t), LimitsError::CpuSeconds),
            (Limits::new(m, 1, 1, Bytes(0)), LimitsError::Tmpfs),
        ];
        for (made, want) in cases {
            assert_eq!(made.err(), Some(want));
        }
        let ok = Limits::new(m, 1, 1, t).expect("usable");
        assert_eq!(
            (ok.memory(), ok.pids(), ok.cpu_seconds(), ok.tmpfs()),
            (m, 1, 1, t)
        );
    }

    /// The same checks apply to limits read from a file: a bad number is an error there, not a
    /// `Limits` that fails later.
    #[test]
    fn limits_read_from_a_file_are_checked_too() {
        let read = |text: &str| serde_json::from_str::<Limits>(text);
        assert_eq!(read("{}").expect("defaults"), Limits::default());
        let set = read(r#"{"pids": 8}"#).expect("one field");
        assert_eq!(set.pids(), 8);
        assert_eq!(set.cpu_seconds(), Limits::default().cpu_seconds());
        for bad in [r#"{"pids": 0}"#, r#"{"memory": 4097}"#, r#"{"tmpfs": 0}"#] {
            assert!(read(bad).is_err(), "{bad}");
        }
    }
}
