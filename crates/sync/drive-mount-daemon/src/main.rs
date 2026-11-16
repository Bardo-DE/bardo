//! bardo-drive-mount: listens on its socket (passed by `bardo-drive-mount.socket`, or bound at
//! `$XDG_RUNTIME_DIR/bardo/drive-mount.sock` when started by hand) and shows drives in `~/Cloud`.

use base_xdg::Xdg;
use drive_mount_daemon::server::{self, Fusermount, Helper};
use rustix::fs::{FileType, fstat};
use rustix::io::{FdFlags, fcntl_setfd};
use std::io;
use std::os::unix::net::UnixListener;
use std::process::ExitCode;

fn main() -> ExitCode {
    let xdg = Xdg::process();
    let Some(home) = xdg.home() else {
        eprintln!("bardo-drive-mount: there is no home folder (HOME)");
        return ExitCode::FAILURE;
    };
    let runtime = xdg.runtime_dir();
    let activated = match activated() {
        Ok(activated) => activated,
        Err(why) => {
            eprintln!("bardo-drive-mount: the socket passed is no use: {why}");
            return ExitCode::FAILURE;
        }
    };
    let listener = match server::listener(activated, runtime.as_deref()) {
        Ok(listener) => listener,
        Err(why) => {
            eprintln!("bardo-drive-mount: not listening: {why}");
            return ExitCode::FAILURE;
        }
    };
    let helper = Helper::new(home.join("Cloud"), Fusermount::new("fusermount3"));
    helper.serve(&listener)
}

/// The listening socket systemd passed, when it passed one to this process. `listenfd` checks
/// `LISTEN_PID` and takes the fd; the close-on-exec flag and the socket check stay here, because
/// `fusermount3` must not inherit the fd and this is a privileged helper.
fn activated() -> io::Result<Option<UnixListener>> {
    if std::env::var("LISTEN_FDS").ok().as_deref() != Some("1") {
        return Ok(None);
    }
    let Some(listener) = listenfd::ListenFd::from_env().take_unix_listener(0)? else {
        return Ok(None);
    };
    fcntl_setfd(&listener, FdFlags::CLOEXEC)?;
    let stat = fstat(&listener)?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::Socket {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the fd systemd passed is not a socket",
        ));
    }
    Ok(Some(listener))
}
