//! The bar clock: the minute the service publishes, and the writing of a time of day.

mod command;
mod format;
mod model;

pub use command::ClockCommand;
pub use format::{format_clock, time_digits, time_of_day, time_of_minute, until_next_minute};
pub use model::{ClockHours, ClockMinute};

/// The bar clock as a service.
///
/// The bar clock as a service: publishes the wall clock's local time now, then at every
/// minute boundary (the system clock reads the zone afresh, so a time-zone change shows at the
/// next minute, or at once on `Refresh`).
#[derive(Debug)]
pub enum ClockService {}

impl crate::service::Service for ClockService {
    const NAME: &'static str = "clock";
    type State = ClockMinute;
    type Command = ClockCommand;
}
