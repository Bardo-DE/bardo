//! One connection's descriptor and the two calls made on it: send a message, read a packet.

use mouse_gestures_proto::{self as protocol, ToDaemon, ToHost};
use rustix::fd::{AsFd, OwnedFd};
use rustix::io::Errno;
use rustix::net::{RecvFlags, SendFlags};
use service_locator::guard::{Claim, Origin};

/// A connected subscriber: its descriptor and what it showed itself to be when it connected.
#[derive(Debug)]
pub(super) struct Conn {
    pub(super) fd: OwnedFd,
    origin: Origin,
}

/// What one read of a connection found.
#[derive(Debug)]
pub(super) enum Read {
    /// A packet: its message, or why it is none.
    Packet(Result<ToDaemon, String>),
    /// Interrupted: ask again.
    Retry,
    /// Nothing more is queued.
    Empty,
    /// The peer closed, or reading failed for good.
    Closed,
}

impl Conn {
    pub(super) fn new(fd: OwnedFd, origin: Origin) -> Self {
        Self { fd, origin }
    }

    /// What the connection claims right now (a shell whose process has exited claims nothing).
    pub(super) fn claim(&self) -> Claim {
        self.origin.claim()
    }

    /// Sends `m`. False if the peer is gone; a full queue drops the message only.
    pub(super) fn send(&self, m: &ToHost) -> bool {
        match rustix::net::send(
            self.fd.as_fd(),
            &protocol::encode(m),
            SendFlags::DONTWAIT | SendFlags::NOSIGNAL,
        ) {
            Ok(_) | Err(Errno::AGAIN) => true,
            Err(_) => false,
        }
    }

    /// Reads one packet into `buf`, which holds one byte more than the largest packet allowed so
    /// that an oversize one is seen as such.
    pub(super) fn read(&self, buf: &mut [u8]) -> Read {
        match rustix::net::recv(&self.fd, &mut *buf, RecvFlags::DONTWAIT) {
            Ok((0, _)) => Read::Closed,
            Ok((n, _)) => Read::Packet(protocol::decode(&buf[..n]).map_err(|e| e.to_string())),
            Err(Errno::AGAIN) => Read::Empty,
            Err(Errno::INTR) => Read::Retry,
            Err(_) => Read::Closed,
        }
    }
}
