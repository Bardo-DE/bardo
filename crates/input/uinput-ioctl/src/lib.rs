//! The uinput ioctls that `input-linux` cannot do for us, behind safe calls.
//!
//! `input-linux-sys` 0.9.0 declares `UI_SET_PHYS` with a one-byte argument size, but the kernel's
//! request is `_IOW('U', 108, char*)`, so its number carries the size of a pointer. The kernel does
//! not know the one-byte number and every call fails with `EINVAL`, so `UInputHandle::set_phys`
//! cannot work. This crate makes the call with the right number. Everything else about a uinput
//! device (the bits, setup, create, writes) stays with `input-linux`, which has no `unsafe` of ours.
#![expect(
    unsafe_code,
    reason = "this crate issues the uinput ioctl through rustix's unsafe typed-ioctl call"
)]

use rustix::fd::AsFd;
use rustix::ioctl::{self, IntegerSetter, Opcode, opcode};
use std::ffi::{CStr, c_char, c_void};
use std::io;

/// `UI_SET_PHYS`: `_IOW('U', 108, char*)`.
const UI_SET_PHYS: Opcode = opcode::write::<*const c_char>(b'U', 108);

/// Sets the physical-location text of a uinput device (the `phys` file in sysfs).
///
/// Call it on the `/dev/uinput` descriptor before the device is set up and created; the kernel
/// refuses it once the device exists. The kernel copies the text during the call, so `phys` need
/// not outlive it.
///
/// # Errors
///
/// Returns the error the kernel gives, for example `ENOTTY` when `fd` is not a uinput node,
/// `EINVAL` for text that is too long, or `EINVAL` after the device is created.
pub fn set_phys(fd: impl AsFd, phys: &CStr) -> io::Result<()> {
    // SAFETY: linux/uinput.h declares UI_SET_PHYS as _IOW('U', 108, char*), and the kernel's
    // handler takes the argument as a `const char __user *` it only reads, up to the terminating
    // NUL. The opcode is built for exactly a pointer argument, the pointer is that of a
    // NUL-terminated `CStr` that outlives the call, and nothing is written through it.
    unsafe {
        ioctl::ioctl(
            fd,
            IntegerSetter::<UI_SET_PHYS>::new_pointer(phys.as_ptr().cast_mut().cast::<c_void>()),
        )
    }
    .map_err(io::Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    #[test]
    fn the_opcode_matches_the_kernel_header() {
        // `_IOW('U', 108, char*)` with an 8-byte pointer: dir 1, size 8, type 'U', number 108.
        assert_eq!(UI_SET_PHYS, 0x4008_556c);
    }

    #[test]
    fn a_non_uinput_descriptor_is_refused() {
        let null = std::fs::File::open("/dev/null").expect("open /dev/null");
        assert!(set_phys(&null, c"input_broker/test").is_err());
    }
}
