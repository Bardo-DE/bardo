//! The hidraw ioctls, behind safe calls.
//!
//! No safe crate declares `HIDIOCSFEATURE`, so it goes through rustix's typed ioctl pattern; the
//! only unsafety is the promise that the opcode and the argument type match the kernel's
//! declaration, and the block states which declaration it matches.
#![expect(
    unsafe_code,
    reason = "this crate issues the hidraw ioctl through rustix's unsafe typed-ioctl call"
)]

use rustix::fd::AsFd;
use rustix::ioctl::{self, Opcode, Updater, opcode};
use std::io;

/// `HIDIOCSFEATURE(3)`: send a 3-byte feature report.
const HIDIOCSFEATURE_3: Opcode = opcode::read_write::<[u8; 3]>(b'H', 0x06);

/// Sends a 3-byte feature report to a hidraw device, for example `F1 02 01`, which enables
/// multitouch reports on a Magic Mouse.
///
/// `report` is the whole report, report id first. The length is part of the ioctl number, so it
/// is fixed at three bytes (a generic length cannot be an opcode on stable Rust; add another
/// constant here when a device needs one). The kernel may write into the buffer, so the call
/// works on a copy.
///
/// # Errors
///
/// Returns the error the kernel gives, for example when `fd` is not a hidraw node, was opened
/// read-only, or the device refuses the report.
pub fn send_feature(fd: impl AsFd, report: &[u8; 3]) -> io::Result<()> {
    let mut buf = *report;
    // SAFETY: linux/hidraw.h declares HIDIOCSFEATURE(len) as _IOC(_IOC_WRITE|_IOC_READ, 'H',
    // 0x06, len) taking a `char[len]` buffer; the opcode is built for exactly `[u8; 3]` and the
    // buffer is that array, borrowed mutably for the call.
    unsafe { ioctl::ioctl(fd, Updater::<HIDIOCSFEATURE_3, [u8; 3]>::new(&mut buf)) }
        .map_err(io::Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_opcode_matches_the_kernel_header() {
        // Value from linux/hidraw.h on x86_64.
        assert_eq!(HIDIOCSFEATURE_3, 0xc003_4806);
    }

    #[test]
    fn a_non_hidraw_descriptor_is_refused() {
        let null = std::fs::File::open("/dev/null").expect("open /dev/null");
        assert!(send_feature(&null, &[0xF1, 0x02, 0x01]).is_err());
    }
}
