//! Embeds the catalog stoker ships, as it is in this tree, for `testkit::pinned_stoker_catalog`.
//!
//! A test runs where the build's sources may not be (an archive run in a jail binds only the
//! repo and the target directory), so the files go into the test binary itself. stoker lives in
//! this monorepo at `crates/stoker`, found from this crate's manifest directory. The embedded
//! files are named by a hash of their content, so a changed catalog never reuses an old copy.

use std::path::{Path, PathBuf};

fn main() {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let catalog = manifest.join("../../models/catalog");
    println!("cargo:rerun-if-changed={}", catalog.display());
    let files = catalog_files(&catalog);
    let mut code = String::from("&[\n");
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for (name, path) in &files {
        println!("cargo:rerun-if-changed={}", path.display());
        code.push_str(&format!(
            "    ({name:?}, include_str!({:?})),\n",
            path.display().to_string()
        ));
        let bytes = name.bytes().chain(std::fs::read(path).unwrap_or_default());
        for byte in bytes {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    code.push_str("]\n");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("out dir"));
    std::fs::write(out.join("stoker_catalog.rs"), code).expect("write the embedded catalog");
    println!("cargo:rustc-env=BARDO_ENGINE_CATALOG_REV={hash:016x}");
}

/// The catalog's `.toml` files, by name.
fn catalog_files(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut files: Vec<(String, PathBuf)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .filter_map(|path| Some((path.file_name()?.to_str()?.to_owned(), path)))
        .collect();
    files.sort();
    files
}
