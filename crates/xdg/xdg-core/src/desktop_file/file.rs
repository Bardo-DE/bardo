//! A parsed desktop file: its groups in file order.

use super::error::DesktopFileError;
use super::group::DesktopGroup;
use super::parse::{Line, classify};
use base_fs::ReadCap;
use std::collections::BTreeMap;

/// The group a desktop entry's own keys live in.
const ENTRY_GROUP: &str = "Desktop Entry";

/// The largest desktop file read by default. Real entries are a few KiB and the ones carrying
/// every translation stay well under 100 KiB, so a file over this is not a launcher.
pub const DESKTOP_ENTRY_CAP: ReadCap = ReadCap::kib(256);

/// A `.desktop` file (or any file in the same key file format, such as an icon theme's
/// `index.theme`) as groups of keys. See the module doc for what is skipped and how duplicates
/// are read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DesktopFile {
    groups: Vec<DesktopGroup>,
    skipped: usize,
}

impl DesktopFile {
    /// The file `text` holds; [`DesktopFileError`] when `text` is longer than `cap`
    /// ([`DESKTOP_ENTRY_CAP`] suits a desktop entry).
    ///
    /// # Errors
    /// [`DesktopFileError`] when `text` is longer than `cap`.
    pub fn parse(text: &str, cap: ReadCap) -> Result<DesktopFile, DesktopFileError> {
        refuse_over(text.len(), cap)?;
        Ok(DesktopFile::from_text(text))
    }

    /// The file `bytes` hold, with anything that is not UTF-8 replaced; refused over `cap` as
    /// [`DesktopFile::parse`] does.
    ///
    /// # Errors
    /// [`DesktopFileError`] when `bytes` is longer than `cap`.
    pub fn from_bytes(bytes: &[u8], cap: ReadCap) -> Result<DesktopFile, DesktopFileError> {
        refuse_over(bytes.len(), cap)?;
        Ok(DesktopFile::from_text(&String::from_utf8_lossy(bytes)))
    }

    /// The file `text` holds; a byte order mark at the start is ignored.
    fn from_text(text: &str) -> DesktopFile {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut file = DesktopFile::default();
        let mut index: BTreeMap<String, usize> = BTreeMap::new();
        let mut current: Option<usize> = None;
        for line in text.lines() {
            match classify(line) {
                Line::Blank => {}
                Line::Header(name) => {
                    let at = *index.entry(name.to_owned()).or_insert_with(|| {
                        file.groups.push(DesktopGroup::named(name));
                        file.groups.len() - 1
                    });
                    current = Some(at);
                }
                Line::Pair { key, locale, value } => {
                    match current.and_then(|at| file.groups.get_mut(at)) {
                        Some(group) => group.set(key, locale, value),
                        None => file.skipped += 1,
                    }
                }
                Line::Skipped => file.skipped += 1,
            }
        }
        file
    }

    /// The group called `name`, if the file has one.
    #[must_use]
    pub fn group(&self, name: &str) -> Option<&DesktopGroup> {
        self.groups.iter().find(|group| group.name() == name)
    }

    /// The `[Desktop Entry]` group: the entry itself. Its `Desktop Action` groups are named by its
    /// `Actions` list and found with [`DesktopFile::group`].
    #[must_use]
    pub fn entry(&self) -> Option<&DesktopGroup> {
        self.group(ENTRY_GROUP)
    }

    /// Every group, in the order the file first names them.
    pub fn groups(&self) -> impl Iterator<Item = &DesktopGroup> {
        self.groups.iter()
    }

    /// How many lines were skipped as neither a comment, a group header nor a `Key=value` pair,
    /// or as a pair before any group.
    #[must_use]
    pub fn skipped_lines(&self) -> usize {
        self.skipped
    }
}

/// Refuses a text of `size` bytes over `cap`.
fn refuse_over(size: usize, cap: ReadCap) -> Result<(), DesktopFileError> {
    let size = u64::try_from(size).unwrap_or(u64::MAX);
    if size > cap.limit() {
        return Err(DesktopFileError { size, cap });
    }
    Ok(())
}
