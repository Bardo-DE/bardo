//! Settings files on disk: one program's config directory ([`Store`]: lenient load, atomic
//! save that keeps unknown keys and comments, a debounced directory watch), the config tree
//! it sits in ([`ConfigRoot`]), changing one key in place ([`SettingsWriter`]), the files a
//! program's schema is written to and discovered from, and the one directory watch and debounce
//! every settings-file watcher is made of ([`watch_file`]).
//!
//! The reading and writing of the text itself is `settings-store`; this crate is the edge. A watch
//! runs as a task on the [`ui_core::spawner::Spawner`] it is handed: no runtime is named here.

mod damaged;
mod error;
mod locator;
mod root;
mod schema_files;
mod store;
mod text;
mod watch;
mod writer;

pub use damaged::{DamagedFile, Saved, damage, keep_damaged};
pub use error::SettingsError;
pub use locator::{DiskText, Locator, ReadFailure, ReadText};
pub use root::ConfigRoot;
pub use schema_files::{
    data_dirs, data_dirs_from, data_dirs_list_from, data_home_from, data_search_from, discover,
    maybe_write_schema, write_schema, write_schema_part,
};
pub use store::Store;
pub use text::{FileText, SETTINGS_CAP, file_text};
pub use watch::{
    DEBOUNCE, FileWatcher, LocatedWatch, Settled, Watch, WatchState, watch_file,
    watch_file_on_thread,
};
pub use writer::SettingsWriter;
