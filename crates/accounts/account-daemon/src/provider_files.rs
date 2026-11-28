//! Reads the provider files of the daemon's directories: the one place accountd opens them, so
//! the families crate takes the files as data.

use account_families::set::{Layer, ProviderDir, ProviderFile};
use std::path::{Path, PathBuf};

/// The `.toml` files of each directory in `layers`, in file-name order; a directory that cannot be
/// listed (missing, or not readable) has none, and a file that cannot be read keeps its error.
pub(crate) fn read_provider_dirs(layers: &[(Layer, PathBuf)]) -> Vec<ProviderDir> {
    layers
        .iter()
        .map(|(layer, dir)| ProviderDir {
            layer: *layer,
            files: read_dir(dir),
        })
        .collect()
}

fn read_dir(dir: &Path) -> Vec<ProviderFile> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir).map_or_default(|entries| {
        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
            .collect()
    });
    paths.sort();
    paths
        .into_iter()
        .map(|path| ProviderFile {
            text: std::fs::read_to_string(&path),
            path,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_testkit::Scratch;

    #[test]
    fn toml_files_come_in_name_order_and_a_missing_directory_is_empty() {
        let dir = Scratch::new("provider-files");
        std::fs::write(dir.join("b.toml"), "b").expect("write");
        std::fs::write(dir.join("a.toml"), "a").expect("write");
        std::fs::write(dir.join("notes.txt"), "no").expect("write");
        let read = read_provider_dirs(&[
            (Layer::Shipped, dir.to_path_buf()),
            (Layer::Named, PathBuf::from("/nonexistent/providers")),
        ]);
        let [shipped, named] = read.as_slice() else {
            panic!("two directories");
        };
        let files: Vec<_> = shipped
            .files
            .iter()
            .map(|f| f.path.file_name().expect("name").to_owned())
            .collect();
        assert_eq!(files, ["a.toml", "b.toml"]);
        assert_eq!(shipped.files[0].text.as_deref().ok(), Some("a"));
        assert!(named.files.is_empty());
    }
}
