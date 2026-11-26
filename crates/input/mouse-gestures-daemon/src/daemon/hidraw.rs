//! The mouse's hidraw node: opening it, enabling multitouch, reading one report. `read_up_to`
//! also serves the evdev node, whose reads are runs of events.

use super::source::Purpose;
use crate::time::Mono;
use rustix::event::{PollFd, PollFlags, poll};
use rustix::fd::OwnedFd;
use rustix::fs::{Mode, OFlags};
use rustix::time::{ClockId, Timespec, clock_gettime};
use std::io;
use std::path::Path;
use std::time::Duration;

/// The feature report that turns on multitouch reports (R12).
pub const ENABLE_MULTITOUCH: [u8; 3] = [0xf1, 0x02, 0x01];
/// The largest read the mouse produces.
pub const READ_LEN: usize = 64;

/// Now on `CLOCK_MONOTONIC`.
pub fn now() -> Mono {
    let t = clock_gettime(ClockId::Monotonic);
    Mono(
        u64::try_from(t.tv_sec).unwrap_or(0) * 1_000_000_000
            + u64::try_from(t.tv_nsec).unwrap_or(0),
    )
}

/// Opens the node for `purpose`: a run opens it read-write (falling back to read-only), a
/// recording read-only. Whether it came out writable, so multitouch can be enabled, is the second
/// half of the result.
pub fn open(path: &Path, purpose: Purpose) -> io::Result<(OwnedFd, bool)> {
    let flags = OFlags::CLOEXEC | OFlags::NONBLOCK;
    if purpose == Purpose::Run
        && let Ok(fd) = rustix::fs::open(path, flags | OFlags::RDWR, Mode::empty())
    {
        return Ok((fd, true));
    }
    Ok((
        rustix::fs::open(path, flags | OFlags::RDONLY, Mode::empty())?,
        false,
    ))
}

/// Sends `F1 02 01`; the kernel sends it at probe and resume too, so repeating it is harmless.
pub fn enable_multitouch(fd: &OwnedFd) -> io::Result<()> {
    hidraw_ioctl::send_feature(fd, &ENABLE_MULTITOUCH)
}

/// Waits up to `timeout` for one read of at most `len` bytes: `Ok(None)` when nothing came.
pub fn read_up_to(fd: &OwnedFd, timeout: Duration, len: usize) -> io::Result<Option<Vec<u8>>> {
    let ts = Timespec {
        tv_sec: i64::try_from(timeout.as_secs()).unwrap_or(i64::MAX),
        tv_nsec: i64::from(timeout.subsec_nanos()),
    };
    let mut fds = [PollFd::new(fd, PollFlags::IN)];
    match poll(&mut fds, Some(&ts)) {
        Ok(0) => return Ok(None),
        Ok(_) => {}
        Err(e) if e == rustix::io::Errno::INTR => return Ok(None),
        Err(e) => return Err(e.into()),
    }
    let mut buf = vec![0u8; len];
    match rustix::io::read(fd, &mut buf) {
        Ok(0) => Err(io::ErrorKind::UnexpectedEof.into()),
        Ok(n) => Ok(Some(buf[..n].to_vec())),
        Err(rustix::io::Errno::AGAIN) => Ok(None),
        Err(e) => Err(e.into()),
    }
}
