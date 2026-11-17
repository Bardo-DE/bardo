//! The platform edge the desktop uses from the viewer's slice: single instance (the request the
//! launcher hands the viewer) and the shared thumbnail cache. Each is a trait with a portable or
//! Linux implementation and a fake (`testing`, behind the `testing` feature). This is the only
//! crate that names D-Bus here; the directories and the session bus come in through [`Env`].
//!
//! Two implementation modules. `portable` is built everywhere and calls no desktop service:
//! single instance over a per-user socket and the thumbnail cache. `linux` is the Linux desktop's
//! own D-Bus single instance and agent intents, built on Linux with the `desktop` feature
//! (opt-in); without the feature this crate does not depend on zbus.
//!
//! A viewer claims the name once at start; a second launch forwards its request and leaves:
//! ```no_run
//! use anyview_platform::portable::LatchkeyInstance;
//! use anyview_platform::{Claim, Instance, PlatformError, Request};
//!
//! # async fn start() -> Result<(), PlatformError> {
//! match LatchkeyInstance::new().claim(&Request::Open(Vec::new())).await? {
//!     Claim::Primary(primary) => {
//!         // Blocks; an async program reads it on a thread of its own.
//!         for request in primary {
//!             println!("{request:?}");
//!         }
//!     }
//!     Claim::Forwarded => {}
//! }
//! # Ok(())
//! # }
//! ```
//!
//! Every public item is reached from this root once, except the implementations, which are
//! reached through `portable`, `linux` and `testing`.

#![warn(missing_docs)]

mod env;
mod error;
mod handoff;
mod instance;
mod thumbnail;
mod uri;

#[cfg(all(feature = "desktop", target_os = "linux"))]
mod desktop;
#[cfg(all(feature = "desktop", target_os = "linux"))]
pub use desktop::linux;
pub mod portable;
#[cfg(any(test, feature = "testing"))]
pub mod testing;

pub use env::{BusRoute, Dirs, Env};
pub use error::{IoOp, PlatformError};
pub use instance::{Claim, Handoff, Instance, Primary, Request};
pub use thumbnail::{ThumbPixels, ThumbSize, ThumbnailCache};
pub use uri::file_uri;
