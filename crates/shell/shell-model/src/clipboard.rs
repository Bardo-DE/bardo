//! The clipboard history: entries, what is kept and what is skipped, and the pure step over copies
//! and policy.

mod command;
mod model;
mod offer;
mod step;

pub use command::{ClipEffect, ClipEvent, ClipboardCommand, Skipped};
pub use model::{
    ClipContent, ClipEntry, ClipImage, ClipPolicy, ClipProtocol, ClipState, ClipStatus, ClipText,
    Clipboard, EntryCap, HistoryMode, ImageCap, clip_image,
};
pub use offer::{ImagesKept, MimePick, SECRET_HINTS, offer_verdict};
pub use step::step;

/// The clipboard history as a service, on `env`'s compositor. Passive until the daemon's
/// first policy.
#[derive(Debug)]
pub enum ClipboardService {}

impl crate::service::Service for ClipboardService {
    const NAME: &'static str = "clipboard";
    type State = Clipboard;
    type Command = ClipboardCommand;
}
