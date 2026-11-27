//! The service's tests: a fake kernel, a fake authority and sockets in scratch folders. No device,
//! no `/run`, no bus, no real `/proc`. The tests that make a real device are the `live` module.

mod daemon;
mod identity;
mod live;
mod misc;
mod pending;
mod policy;
mod real_authority;
mod serve;
mod session;
mod slots;
mod support;
