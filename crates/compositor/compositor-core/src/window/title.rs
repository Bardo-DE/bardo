//! A window's title.

/// The title a client set. App-authored text: wherever it leaves the compositor it is untrusted, and
/// The compositor itself never draws it before `compositor-chrome` (P3).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WindowTitle(pub String);
