//! All integration tests, one binary. No test here sets process-global state, so none needs its
//! own process.

mod decode;
mod evdev_parity;
mod gestures_replay;
mod pad_strokes;
mod recorded_calibration;
mod reference_cases;
mod support;
