//! The wallpaper: where it comes from and every output's cached images, as the wallpaper and lock
//! surfaces draw them.

mod cache;
mod command;
mod geometry;
mod model;

pub use cache::{CACHE_DIR, backdrop_file, cache_file_name, source_hash};
pub use command::{WallpaperCommand, WallpaperPicture};
pub use geometry::{CoverWindow, cover_window};
pub use model::{WallpaperImage, WallpaperSource, Wallpapers};

/// The wallpapers as a service: resized on the blocking pool, published as cached paths.
/// The old source file is read under `env`'s config directory, the resized copies kept in its cache.
#[derive(Debug)]
pub enum WallpaperService {}

impl crate::service::Service for WallpaperService {
    const NAME: &'static str = "wallpaper";
    type State = Wallpapers;
    type Command = WallpaperCommand;
}
