//! bardo-drive-mount's tests: requests over real sockets, with a fake mount step (the jail has no
//! /dev/fuse and no fusermount3).

mod common;
mod dist;
mod helper;
mod socket;
