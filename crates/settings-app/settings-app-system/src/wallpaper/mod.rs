//! The pictures the Wallpaper page offers, read from the person's folders, and the small pictures
//! of their tiles.

mod disk;
mod thumbs;

pub use disk::OnDisk;
pub use thumbs::{ThumbFault, Thumbnails};
