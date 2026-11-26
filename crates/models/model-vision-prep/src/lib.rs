//! Image preparation for vision models: how big the model should see a frame, what that costs
//! in tokens, and the map from the model's coordinates back to the window's.
//!
//! ```
//! use computer_use_action::{Coord, DeviceSize, Size};
//! use model_vision_prep::{ImageTokens, PatchFactor, PixelCount, ResizeRule, fit, image_tokens};
//!
//! // A Qwen-VL style rule: sides round to a multiple of 28.
//! let rule = ResizeRule::SmartResize {
//!     factor: PatchFactor(28),
//!     min_pixels: PixelCount(3_136),
//!     max_pixels: PixelCount(12_845_056),
//! };
//! let shown = fit(DeviceSize { w: 1280, h: 800 }, &rule).unwrap();
//! assert_eq!(shown, Size::new(Coord(1288), Coord(812)));
//! assert_eq!(image_tokens(&rule, shown), ImageTokens(1334));
//! ```
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

mod frame_map;
mod pixels;
mod rule;

pub use frame_map::{FrameMap, MapError};
#[cfg(feature = "pixels")]
pub use pixels::prepare;
pub use pixels::{
    Encoding, JpegQuality, JpegQualityError, MediaType, PrepError, PreparedImage, RawFrame,
};
pub use rule::{FitError, ImageTokens, PatchFactor, PixelCount, ResizeRule, fit, image_tokens};
