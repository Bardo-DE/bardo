//! The notification parts' geometry tokens (design/13 section 13.3.6).
//!
//! They are the banner's width,
//! floor, padding and icon, the close button, the gap between stacked banners, the offset of a
//! group's layers, the notification center's width and where the banner column sits, each a
//! tuned token a look or the person's style sheet may move (its `input`, the default behind
//! it). The layer surfaces and the icon's raster are sized from them before a layout
//! (`shell_ui_kit::notification_card::NotificationTokens`).

use ui_components::style::tokens::token::Token;
use ui_components::words::Word;

/// One notification geometry token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word, Token)]
#[token(prefix = "", kind = tuned)]
pub enum NotificationToken {
    /// `--notifications-banner-width` (320).
    #[token(
        name = "notifications-banner-width",
        input = "--notifications-banner-width-px",
        value = "320px"
    )]
    BannerWidth,
    /// `--notifications-banner-min-height` (48).
    #[token(
        name = "notifications-banner-min-height",
        input = "--notifications-banner-min-height-px",
        value = "48px"
    )]
    BannerMinHeight,
    /// `--notifications-banner-padding` (10).
    #[token(
        name = "notifications-banner-padding",
        input = "--notifications-banner-padding-px",
        value = "10px"
    )]
    BannerPadding,
    /// `--notifications-icon` (28): the app icon's side.
    #[token(
        name = "notifications-icon",
        input = "--notifications-icon-px",
        value = "28px"
    )]
    Icon,
    /// `--notifications-close` (18): the close button's diameter.
    #[token(
        name = "notifications-close",
        input = "--notifications-close-px",
        value = "18px"
    )]
    Close,
    /// `--notifications-stack-gap` (8): between two banners.
    #[token(
        name = "notifications-stack-gap",
        input = "--notifications-stack-gap-px",
        value = "8px"
    )]
    StackGap,
    /// `--notifications-group-offset` (4): how far each of a
    /// group's layers shows below the one above it.
    #[token(
        name = "notifications-group-offset",
        input = "--notifications-group-offset-px",
        value = "4px"
    )]
    GroupOffset,
    /// `--notifications-center-width` (320).
    #[token(
        name = "notifications-center-width",
        input = "--notifications-center-width-px",
        value = "320px"
    )]
    CenterWidth,
    /// `--shell-notifications-position-right`: a banner's gap from the output's right edge (8).
    #[token(
        name = "shell-notifications-position-right",
        input = "--shell-notifications-position-right-px",
        value = "8px"
    )]
    PositionRight,
    /// `--shell-notifications-position-top-gap`: a banner's gap from the bar's reserve (8).
    #[token(
        name = "shell-notifications-position-top-gap",
        input = "--shell-notifications-position-top-gap-px",
        value = "8px"
    )]
    PositionTopGap,
}
