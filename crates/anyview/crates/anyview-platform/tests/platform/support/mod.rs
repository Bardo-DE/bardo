//! The private session bus the tests of this crate share: `anyview_platform::testing::PrivateBus`.

// Each test crate uses some of these helpers, so an unused one is not dead code.
#![allow(
    dead_code,
    reason = "helpers shared by the test files; each file uses some of them"
)]

pub use anyview_platform::testing::PrivateBus;
