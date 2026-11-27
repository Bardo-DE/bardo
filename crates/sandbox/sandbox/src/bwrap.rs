//! The real sandbox: bubblewrap (`bwrap`, LGPL) run as a separate process and never linked. It
//! builds the command line for one run and starts it; `bwrap_job` owns the running process.
//!
//! What a run gets: the host's file system read-only, a fresh `/dev` and `/proc`, empty in-memory
//! `/tmp`, `/run`, `/home`, `/root`, `/mnt` and the like (so home directories, the session bus
//! socket and other users' files are not there to read), the working directory bound writable and
//! nothing else writable, new user, pid, ipc, uts, cgroup and network namespaces (no interface at
//! all, not even loopback), no capabilities, a new session (its own process group), a cleared
//! environment, and death with its parent. With the host's network, the resolver files under `/run`
//! that `/etc/resolv.conf` may point at are bound back read-only (`RESOLVER_FILES`), or names would
//! not resolve. FINDINGS.md "Sandboxed shell" lists what that does not
//! stop.

use crate::apparmor::{RESTRICT_USERNS, refusal_at};
use crate::bwrap_job::BwrapJob;
use crate::fds::inherited_fds;
use crate::launch::{Enforcement, Env, Feed, Launcher, UserSession};
use base_path::AbsPath;
use sandbox_core::{
    AgentRun, CannotSandboxError, Hardening, Profile, RunSpec, Sandbox, SandboxState, StartError,
    agent_bwrap_args, bwrap_args, default_filter,
};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The bubblewrap sandbox.
#[derive(Debug, Clone)]
pub struct BwrapSandbox {
    program: PathBuf,
    launcher: Launcher,
    /// The seccomp filter bubblewrap is given, when it is on.
    filter: Option<Vec<u8>>,
    hardening: Hardening,
}

/// Fails closed where a descriptor would leak into the sandbox, unless the person allowed it.
fn no_leaks(allowed: bool) -> Result<(), StartError> {
    if allowed {
        Ok(())
    } else {
        match inherited_fds() {
            Ok(found) if found.is_empty() => Ok(()),
            _ => Err(StartError::InheritedFds),
        }
    }
}

impl BwrapSandbox {
    /// The `bwrap` program this sandbox runs.
    #[must_use]
    pub fn program(&self) -> &Path {
        &self.program
    }

    /// How the resource limits are enforced for this sandbox.
    #[must_use]
    pub fn enforcement(&self) -> Enforcement {
        self.launcher.enforcement()
    }

    /// The profile this sandbox adds to every command line: scratch directory sizes.
    #[must_use]
    pub fn profile(&self) -> Profile {
        let mut sized = match self.launcher.limits() {
            Some(limits) => Profile::bare().with_tmpfs_size(limits.tmpfs()),
            None => Profile::bare(),
        };
        sized = sized
            .with_opt(self.hardening.opt)
            .covering_secrets_where(|path| Path::new(path).exists());
        for path in &self.hardening.extra_ro {
            sized = sized.with_extra_ro(path.clone());
        }
        match self.filter {
            Some(_) => sized.with_seccomp(),
            None => sized,
        }
    }

    /// Writes the seccomp filter to a new file at `path` (mode 0600; it must not exist), for
    /// `agent_command`. Nothing is written, and `Ok` is returned, when the filter is off.
    ///
    /// # Errors
    /// Returns the I/O error when the file exists already or cannot be created or written.
    pub fn write_filter(&self, path: &AbsPath) -> std::io::Result<()> {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let Some(filter) = &self.filter else {
            return Ok(());
        };
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path.as_str())?;
        file.write_all(filter)
    }

    /// The command that confines the agent process `run`: bubblewrap with the sandbox's own
    /// profile (scratch sizes, seccomp), started under the resource limits, with exactly the
    /// environment of `run`. The caller sets the stdio and spawns it.
    ///
    /// When the seccomp filter is on, `filter` must name a file written with `write_filter`;
    /// without one this is `StartError::FilterFile`, never an unfiltered agent. Every
    /// descriptor the caller opens before spawning must be close-on-exec (see `inherited_fds`).
    ///
    /// # Errors
    /// Returns `StartError::Cannot` when the sandbox cannot confine `run.cwd`,
    /// `StartError::InheritedFds` when a descriptor would leak, and `StartError::FilterFile` when
    /// the seccomp filter is on and no `filter` file was given.
    pub fn agent_command(
        &self,
        run: &AgentRun,
        filter: Option<&AbsPath>,
    ) -> Result<Command, StartError> {
        if let SandboxState::Cannot(why) = self.check(&run.cwd) {
            return Err(StartError::Cannot(why));
        }
        no_leaks(self.hardening.inherited_fds)?;
        let feed = match (&self.filter, filter) {
            (None, _) => Feed::NoFilter,
            (Some(_), Some(file)) => Feed::FromFile(file.clone()),
            (Some(_), None) => return Err(StartError::FilterFile),
        };
        let run = run.clone().keeping_protected(protectable);
        let args = agent_bwrap_args(&run, &self.profile());
        Ok(self
            .launcher
            .command(&self.program, &args, &feed, Env::Exactly(&run.env)))
    }

    /// `detect_with` for the default hardening: cgroup limits, the session of this process.
    ///
    /// # Errors
    /// The same as [`Self::detect_with`].
    pub fn detect(path: &str) -> Result<Self, CannotSandboxError> {
        Self::detect_with(path, &Hardening::default(), &UserSession::from_env())
    }

    /// Finds `bwrap` in the `:`-separated `path`, checks that the kernel lets it make the
    /// namespaces (one throwaway run), and that `hardening` can be applied. Where a limit that
    /// was asked for cannot be applied this is an error, never a weaker sandbox.
    ///
    /// # Errors
    /// Returns `CannotSandboxError` naming why: no `bwrap` on `path`, the kernel or `AppArmor` refusing
    /// the namespaces, the seccomp filter or the resource limits not applicable here, or an
    /// unsupported platform.
    pub fn detect_with(
        path: &str,
        hardening: &Hardening,
        session: &UserSession,
    ) -> Result<Self, CannotSandboxError> {
        if cfg!(not(unix)) {
            return Err(CannotSandboxError::Unsupported);
        }
        let program = path
            .split(':')
            .filter(|d| !d.is_empty())
            .map(|d| Path::new(d).join("bwrap"))
            .find(|p| p.is_file())
            .ok_or(CannotSandboxError::NotInstalled)?;
        if !bwrap_runs(&program, &[], None) {
            return Err(refusal_at(Path::new(RESTRICT_USERNS)));
        }
        let filter = if hardening.seccomp {
            #[expect(
                clippy::map_err_ignore,
                reason = "`CannotSandboxError` is a plain word that is complete by itself: either way no filter can be made here"
            )]
            let bytes = default_filter().map_err(|_| CannotSandboxError::SeccompUnavailable)?;
            if !bwrap_runs(&program, &["--seccomp", "0"], Some(&bytes)) {
                return Err(CannotSandboxError::SeccompUnavailable);
            }
            Some(bytes)
        } else {
            None
        };
        let launcher = Launcher::resolve(path, &hardening.limits, session)?;
        if launcher.limits().is_some()
            && !bwrap_runs(&program, &["--size", "1048576", "--tmpfs", "/tmp"], None)
        {
            return Err(CannotSandboxError::LimitsUnavailable);
        }
        Ok(Self {
            program,
            launcher,
            filter,
            hardening: hardening.clone(),
        })
    }
}

/// Whether `path` exists and is not a symbolic link (judged without following it): a protected
/// path that is not, would make bubblewrap fail or follow the link out of the workspace.
fn protectable(path: &AbsPath) -> bool {
    std::fs::symlink_metadata(path.as_str()).is_ok_and(|m| !m.file_type().is_symlink())
}

/// A pipe already holding `bytes` (a filter is far below the pipe's capacity), read end only.
fn filter_pipe(bytes: &[u8]) -> Result<Stdio, StartError> {
    use std::io::Write;
    let (reader, mut writer) = std::io::pipe().map_err(|e| StartError::Spawn(e.to_string()))?;
    writer
        .write_all(bytes)
        .map_err(|e| StartError::Spawn(e.to_string()))?;
    Ok(Stdio::from(reader))
}

/// One throwaway run of `program` with `extra` arguments, `stdin` as its standard input (empty
/// when none); whether it succeeded.
fn bwrap_runs(program: &Path, extra: &[&str], stdin: Option<&[u8]>) -> bool {
    use std::io::Write;
    let Ok((reader, mut writer)) = std::io::pipe() else {
        return false;
    };
    if let Some(bytes) = stdin
        && writer.write_all(bytes).is_err()
    {
        return false;
    }
    drop(writer);
    Command::new(program)
        .args(["--ro-bind", "/", "/", "--unshare-all", "--die-with-parent"])
        .args(extra)
        .args(["--", "true"])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::from(reader))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// A working directory is usable when it is a real directory at least two levels down (never `/`,
/// never `/home` or `/tmp` themselves), so the writable place is a project and not a system root.
fn usable(cwd: &AbsPath) -> bool {
    let deep = cwd.as_str().matches('/').count() >= 2;
    !cwd.is_root() && deep && Path::new(cwd.as_str()).is_dir()
}

impl Sandbox for BwrapSandbox {
    type Job = BwrapJob;

    fn available(&self) -> SandboxState {
        SandboxState::Ready
    }

    fn check(&self, cwd: &AbsPath) -> SandboxState {
        if usable(cwd) {
            SandboxState::Ready
        } else {
            SandboxState::Cannot(CannotSandboxError::BadWorkingDir)
        }
    }

    fn start(&self, spec: &RunSpec) -> Result<BwrapJob, StartError> {
        if let SandboxState::Cannot(why) = self.check(&spec.cwd) {
            return Err(StartError::Cannot(why));
        }
        no_leaks(self.hardening.inherited_fds)?;
        let args = bwrap_args(spec, &self.profile());
        let (feed, stdin) = match &self.filter {
            None => (Feed::NoFilter, Stdio::null()),
            Some(bytes) => (Feed::FromStdin, filter_pipe(bytes)?),
        };
        let command = self
            .launcher
            .command(&self.program, &args, &feed, Env::Minimal);
        BwrapJob::spawn(command, spec.keep, stdin)
    }
}

/// What startup found: bubblewrap, or the reason there is none. A `Detected::Missing` starts
/// nothing, so a build that finds no sandbox runs no command.
#[derive(Debug, Clone)]
pub enum Detected {
    /// A working bubblewrap.
    Bwrap(BwrapSandbox),
    /// No usable sandbox, for this reason.
    Missing(CannotSandboxError),
}

impl Detected {
    /// Looks for a sandbox in the `:`-separated `path`. Compiled but not run by the doc tests:
    /// it starts a real `bwrap`, which needs unprivileged user namespaces.
    ///
    /// ```no_run
    /// use sandbox::Detected;
    /// use sandbox_core::{Argv, Launch, Shell};
    /// use base_path::AbsPath;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let sandbox = Detected::probe("/usr/bin:/bin");
    /// let mut shell = Shell::new(sandbox);
    /// let argv = Argv::new("ls", &[]).ok_or("no program")?;
    /// let id = shell.create(&Launch::new(argv, AbsPath::parse("/home/u/project")?))?;
    /// println!("{:?}", shell.wait(id)?);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn probe(path: &str) -> Self {
        Self::probe_with(path, &Hardening::default(), &UserSession::from_env())
    }

    /// `probe` for chosen hardening and user session.
    #[must_use]
    pub fn probe_with(path: &str, hardening: &Hardening, session: &UserSession) -> Self {
        match BwrapSandbox::detect_with(path, hardening, session) {
            Ok(found) => Detected::Bwrap(found),
            Err(why) => Detected::Missing(why),
        }
    }
}

impl Sandbox for Detected {
    type Job = BwrapJob;

    fn available(&self) -> SandboxState {
        match self {
            Detected::Bwrap(b) => b.available(),
            Detected::Missing(why) => SandboxState::Cannot(*why),
        }
    }

    fn check(&self, cwd: &AbsPath) -> SandboxState {
        match self {
            Detected::Bwrap(b) => b.check(cwd),
            Detected::Missing(why) => SandboxState::Cannot(*why),
        }
    }

    fn start(&self, spec: &RunSpec) -> Result<BwrapJob, StartError> {
        match self {
            Detected::Bwrap(b) => b.start(spec),
            Detected::Missing(why) => Err(StartError::Cannot(*why)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A protected path is bound only when it is really there and is not a link: a real
    /// directory fits; a missing path and a symbolic link (here to `/etc`) do not.
    #[test]
    fn only_a_real_path_that_is_not_a_link_can_be_protected() {
        let root = tempfile::tempdir().expect("scratch");
        let cwd = root.path().join("proj");
        std::fs::create_dir_all(cwd.join(".git")).expect("git dir");
        std::os::unix::fs::symlink("/etc", cwd.join("link")).expect("link");
        let at =
            |name: &str| AbsPath::parse(cwd.join(name).to_str().expect("utf8")).expect("absolute");
        assert!(protectable(&at(".git")));
        assert!(!protectable(&at("missing")));
        assert!(!protectable(&at("link")));
    }
}
