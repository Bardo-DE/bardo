//! Another program holding a device exclusively, as a second remapper would.

use crate::failure::{Failure, errno, io};
use key_remap_evdev as sys;
use rustix::fd::OwnedFd;
use rustix::fs::{Mode, OFlags};
use std::path::Path;

/// An `EVIOCGRAB` held by the harness until it is dropped or released.
#[derive(Debug)]
pub struct ExternalGrab {
    fd: OwnedFd,
}

impl ExternalGrab {
    /// Opens `node` and grabs it.
    ///
    /// # Errors
    ///
    /// Returns a [`Failure`] when `node` cannot be opened or grabbed.
    pub fn take(node: &Path) -> Result<Self, Failure> {
        let fd = rustix::fs::open(
            node,
            OFlags::RDONLY | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(errno(format!("opening {}", node.display())))?;
        sys::grab(&fd).map_err(|e| Failure::new(format!("grabbing {}: {e}", node.display())))?;
        Ok(Self { fd })
    }

    /// Gives the device back.
    ///
    /// # Errors
    ///
    /// Returns a [`Failure`] when the grab cannot be released.
    pub fn release(self) -> Result<(), Failure> {
        sys::release(&self.fd).map_err(io("releasing the harness's grab"))
    }
}
