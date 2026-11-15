//! `ScrollSettings`: every tunable number of the scroll engine, one field per settings key of
//! design/22 §3.7 `scroll`, with the documented defaults. A host reads its own settings file and
//! hands the engine a `ScrollSettings`; the kit never reads a file itself.
//!
//! Field names are the keys without their section, and with feature `settings` the serde form is
//! the keys' own (a `[scroll]` table deserializes straight into it). Settled values design/22 has
//! no key for (the keyboard line, the spring, the rebound end rule) are constants next to the code
//! that uses them (`scroll::keys`, `scroll::rubber`).

use std::time::Duration;

#[cfg(feature = "settings")]
use serde::{Deserialize, Serialize};

/// A length in logical pixels, as a settings key spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "settings", derive(Serialize, Deserialize))]
pub struct ScrollPx(pub u16);

/// A duration in whole milliseconds, as a settings key spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "settings", derive(Serialize, Deserialize))]
pub struct ScrollMs(pub u16);

/// A ratio in thousandths (`550` is 0.55), as a settings key spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "settings", derive(Serialize, Deserialize))]
pub struct ScrollFraction(pub u16);

/// A dimensionless physics constant (a drag exponent, a speed in px/s).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "settings", derive(Serialize, Deserialize))]
pub struct ScrollScalar(pub f32);

impl ScrollPx {
    /// As an `f64` for the formulas.
    pub fn get(self) -> f64 {
        f64::from(self.0)
    }
}

impl ScrollMs {
    /// As a `Duration`.
    pub fn duration(self) -> Duration {
        Duration::from_millis(u64::from(self.0))
    }

    /// Whole milliseconds.
    pub fn get_ms(self) -> u16 {
        self.0
    }

    /// In seconds, for the formulas.
    pub fn secs(self) -> f64 {
        f64::from(self.0) / 1000.0
    }
}

impl ScrollFraction {
    /// As a ratio.
    pub fn get(self) -> f64 {
        f64::from(self.0) / 1000.0
    }
}

impl ScrollScalar {
    /// As an `f64` for the formulas.
    pub fn get(self) -> f64 {
        f64::from(self.0)
    }
}

/// `scroll.natural`: whether content follows the fingers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "settings", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "settings", serde(rename_all = "snake_case"))]
pub enum NaturalScroll {
    #[default]
    Natural,
    Traditional,
}

/// `scroll.momentum`: whether a flick glides on after the fingers lift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "settings", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "settings", serde(rename_all = "snake_case"))]
pub enum ScrollMomentum {
    #[default]
    On,
    Off,
}

/// `scroll.scrollbars`: when the overlay thumb shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "settings", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "settings", serde(rename_all = "snake_case"))]
pub enum ScrollbarVisibility {
    #[default]
    WhenScrolling,
    /// Never fades, on the legacy 15 px track.
    Always,
}

/// The `scroll` section (design/22 §3.7), plus `scroll.scrollbars` (listed under §3.8).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "settings", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "settings", serde(default))]
pub struct ScrollSettings {
    pub natural: NaturalScroll,
    pub momentum: ScrollMomentum,
    pub momentum_a: ScrollScalar,
    pub momentum_b: ScrollScalar,
    pub momentum_stop_px_s: ScrollScalar,
    pub momentum_start_px_s: ScrollScalar,
    pub momentum_cap_px_s: ScrollScalar,
    pub velocity_window_ms: ScrollMs,
    pub velocity_held_still_ms: ScrollMs,
    pub rubber_band_c: ScrollFraction,
    pub rubber_band_edge_min_px: ScrollPx,
    pub rubber_band_snap_rate: ScrollScalar,
    pub rubber_band_snap_gain: ScrollScalar,
    pub wheel_detent_px: ScrollPx,
    pub wheel_burst_window_ms: ScrollMs,
    /// The most a spun wheel multiplies a detent by (`scroll::accel`); 1 turns it off.
    pub wheel_accel_max: ScrollScalar,
    /// The most a fast lift multiplies the glide by (`scroll::accel`); 1 turns it off.
    pub fling_accel_max: ScrollScalar,
    pub momentum_boost_threshold_px_s: ScrollScalar,
    /// How long the fingers' lift waits for the system's own momentum before the engine glides
    /// itself (`scroll::pad::PointerScroll::Lift`); a host whose system sends none never lifts
    /// that way and is not delayed.
    pub double_scroll_grace_ms: ScrollMs,
    /// The longest a wheel detent's ease takes (the step is still 1000 px/s below it).
    pub wheel_smooth_ms: ScrollMs,
    pub scrollbars: ScrollbarVisibility,
}

impl Default for ScrollSettings {
    fn default() -> ScrollSettings {
        ScrollSettings {
            natural: NaturalScroll::Natural,
            momentum: ScrollMomentum::On,
            momentum_a: ScrollScalar(30.0),
            momentum_b: ScrollScalar(0.7),
            momentum_stop_px_s: ScrollScalar(1.0),
            momentum_start_px_s: ScrollScalar(100.0),
            momentum_cap_px_s: ScrollScalar(12_000.0),
            velocity_window_ms: ScrollMs(80),
            velocity_held_still_ms: ScrollMs(50),
            rubber_band_c: ScrollFraction(550),
            rubber_band_edge_min_px: ScrollPx(10),
            rubber_band_snap_rate: ScrollScalar(12.5),
            rubber_band_snap_gain: ScrollScalar(0.31),
            wheel_detent_px: ScrollPx(60),
            wheel_burst_window_ms: ScrollMs(300),
            wheel_accel_max: ScrollScalar(5.0),
            fling_accel_max: ScrollScalar(2.0),
            momentum_boost_threshold_px_s: ScrollScalar(350.0),
            double_scroll_grace_ms: ScrollMs(150),
            wheel_smooth_ms: ScrollMs(200),
            scrollbars: ScrollbarVisibility::WhenScrolling,
        }
    }
}

impl ScrollSettings {
    /// The macOS feel. The system has already accelerated a notched wheel's clicks, so none is
    /// added (`wheel_accel_max` 1), and a click is 40 px: Chromium's `kScrollbarPixelsPerCocoaTick`
    /// (`ui/events/cocoa/cocoa_event_utils.h`, applied to `NSEvent.deltaY` when the device is not
    /// continuous) and WebKit's `Scrollbar::pixelsPerLineStep` agree. The ease stays under 60 ms,
    /// so a click lands as the system placed it.
    pub fn macos() -> ScrollSettings {
        ScrollSettings {
            wheel_detent_px: ScrollPx(40),
            wheel_accel_max: ScrollScalar(1.0),
            wheel_smooth_ms: ScrollMs(60),
            ..ScrollSettings::default()
        }
    }
}
