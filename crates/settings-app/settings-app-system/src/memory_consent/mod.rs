//! The memory service's controls, as Settings uses them.
//!
//! That is which looser values wait for the person's word and confirming them, and whether memory
//! keeps to its own folders. None starts the service: with it not running nothing waits on anyone, and opening Settings must not start it.

mod bus;
#[cfg(feature = "testing")]
mod fake;
mod proxy;

pub use bus::OnBus;
#[cfg(feature = "testing")]
pub use fake::FakeMemory;
pub use proxy::control;
