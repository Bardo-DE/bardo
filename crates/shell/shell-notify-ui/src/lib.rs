//! The notification surfaces: the banner column, the notification center and the screenshot
//! thumbnail. One directory per surface, each implementing `shell_ui_kit::surface::Surface`; the
//! cards they draw come from `shell_ui_kit::notification_card`.
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

pub mod banners;
mod keyboard;
pub mod notification_center;
pub mod screenshot_thumb;

pub use banners::Banners;
pub use notification_center::NotificationCenter;
pub use screenshot_thumb::ScreenshotThumb;
