//! What every test shares: a scratch directory and Extension folders written into it.

use std::fs;
use std::path::{Path, PathBuf};

/// One test's own directory, removed on drop.
pub struct Scratch(base_testkit::Scratch);

/// A look entry, as a manifest writes it.
pub const LOOK: &str =
    "[[provides]]\ncapability = \"look\"\nstylesheet = \"look.css\"\nstyles_for = 3\n";

/// A file-icons entry.
pub const ICONS: &str = "[[provides]]\ncapability = \"file_icons\"\ndir = \"file-icons\"\n";

impl Scratch {
    pub fn new() -> Scratch {
        Scratch(base_testkit::Scratch::new("extensions"))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    /// A data root named `name` (it stands in for one `$XDG_DATA_HOME` or `$XDG_DATA_DIRS` entry).
    pub fn root(&self, name: &str) -> PathBuf {
        let root = self.0.join(name);
        fs::create_dir_all(&root).unwrap_or_else(|e| panic!("{}: {e}", root.display()));
        root
    }
}

/// A manifest with the envelope filled in and `entries` after it.
pub fn manifest(id: &str, name: &str, version: &str, entries: &str) -> String {
    format!(
        "id = \"{id}\"\nname = \"{name}\"\nprotocol = 1\nversion = \"{version}\"\n\
         licence = \"MIT\"\nsummary = \"A test Extension\"\n\n{entries}"
    )
}

/// The folder `<root>/bardo/extensions/<id>` with `text` as its manifest; returns the folder.
pub fn install(root: &Path, id: &str, text: &str) -> PathBuf {
    let folder = root.join("bardo").join("extensions").join(id);
    fs::create_dir_all(&folder).unwrap_or_else(|e| panic!("{}: {e}", folder.display()));
    write(&folder.join("extension.toml"), text);
    folder
}

/// An Extension of the given `id` (named after it, version 1.0.0) with `entries` in its manifest.
pub fn install_entries(root: &Path, id: &str, entries: &str) -> PathBuf {
    install(root, id, &manifest(id, id, "1.0.0", entries))
}

/// An Extension with a look: its manifest and a small stylesheet.
pub fn install_look(root: &Path, id: &str, name: &str, version: &str) -> PathBuf {
    let folder = install(root, id, &manifest(id, name, version, LOOK));
    write(&folder.join("look.css"), ".ds { --accent: red; }\n");
    folder
}

/// `text` as the file at `path`, its folder made.
pub fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|e| panic!("{}: {e}", parent.display()));
    }
    fs::write(path, text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}
