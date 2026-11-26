//! The parent-death signal.
#![expect(
    unsafe_code,
    reason = "`pre_exec` is unsafe; the closure makes one async-signal-safe `prctl` call"
)]

use rustix::process::Signal;
use std::os::unix::process::CommandExt;
use std::process::Command;

/// Makes the kernel send the child `SIGKILL` when the process that started it dies.
///
/// Call it before `spawn`. It covers the child only, not the child's own children. The kernel
/// counts the parent as dead when the thread that spawned the child exits, so spawn from a thread
/// that lives as long as the process (not a short-lived worker). For a tokio command, pass
/// `command.as_std_mut()`.
///
/// If the call to `prctl` fails in the child, `spawn` returns that error and starts nothing.
pub fn die_with_parent(command: &mut Command) {
    // SAFETY: the closure makes one system call (`prctl`) between fork and exec, which is
    // async-signal-safe, and allocates nothing.
    unsafe {
        command.pre_exec(|| {
            rustix::process::set_parent_process_death_signal(Some(Signal::KILL))
                .map_err(std::io::Error::from)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    #[test]
    fn a_child_spawns_with_the_signal_armed() {
        let mut command = Command::new("true");
        command.stdout(Stdio::null());
        die_with_parent(&mut command);
        let status = command.status().expect("spawn true");
        assert!(status.success());
    }
}
