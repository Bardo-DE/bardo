//! The client's integration tests: one executable, one module per pane, each a flow over the
//! in-memory links of `settings-app-system` (feature `testing`). A bus is never reached: the client is
//! built over `BusSource::Absent`.

mod accounts;
mod bluetooth;
mod computers;
mod displays;
mod engines;
mod memory_consent;
mod picker;
mod running;
mod screen_sharing;
mod spaces;
mod support;
mod updates;
mod wifi;
