//! Unpacks an archive file into a directory, refusing anything that would land outside it or
//! that is larger than the limits. The archive is read as a stream and every byte written is
//! counted: the sizes an archive states about itself are never trusted. The entries are
//! unpacked into a staging folder beside the target, which replaces the target only when the whole
//! archive passed, so a refused archive leaves nothing in the target.

use flate2::read::GzDecoder;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Component, Path, PathBuf};

/// Why an archive was not unpacked.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UnpackFault {
    /// Not a kind docket unpacks (`.tar.gz`, `.tgz` and `.zip` are).
    #[error("this kind of archive cannot be unpacked here")]
    Kind,
    /// An entry would land outside the directory, or is a link.
    #[error("the archive holds a path or link that is not allowed: {0}")]
    Path(String),
    /// The archive holds more, or larger files, than the limits allow.
    #[error("the archive is larger than allowed: {0}")]
    TooLarge(String),
    /// The archive is damaged or the disk refused.
    #[error("the archive could not be unpacked: {0}")]
    Damaged(String),
}

fn damaged(why: impl std::fmt::Display) -> UnpackFault {
    UnpackFault::Damaged(why.to_string())
}

/// How much an archive may hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The most bytes all files together may hold once unpacked.
    pub total: u64,
    /// The most bytes one file may hold.
    pub entry: u64,
    /// The most entries (files and folders) the archive may have.
    pub entries: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            total: 4 << 30,
            entry: 2 << 30,
            entries: 100_000,
        }
    }
}

/// Which archive a name says it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    TarGz,
    Zip,
}

fn kind(name: &str) -> Result<Kind, UnpackFault> {
    let name = name.split(['?', '#']).next().unwrap_or(name);
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        Ok(Kind::TarGz)
    } else if name.ends_with(".zip") {
        Ok(Kind::Zip)
    } else {
        Err(UnpackFault::Kind)
    }
}

/// A relative path with no `..`, root or prefix.
fn inside(path: &Path) -> Option<PathBuf> {
    path.components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
        .then(|| path.to_owned())
}

/// What has been unpacked so far, against the limits.
struct Budget {
    limits: Limits,
    entries: usize,
    written: u64,
}

impl Budget {
    fn new(limits: Limits) -> Self {
        Self {
            limits,
            entries: 0,
            written: 0,
        }
    }

    /// Counts one more entry.
    fn entry(&mut self) -> Result<(), UnpackFault> {
        self.entries += 1;
        if self.entries > self.limits.entries {
            return Err(UnpackFault::TooLarge(format!(
                "more than {} entries",
                self.limits.entries
            )));
        }
        Ok(())
    }

    /// Writes `from` into the new file at `to`, counting the bytes actually written and stopping
    /// at the first byte over a limit.
    fn write(&mut self, from: &mut impl Read, to: &Path) -> Result<(), UnpackFault> {
        let room = self
            .limits
            .entry
            .min(self.limits.total.saturating_sub(self.written));
        let mut file = File::create(to).map_err(damaged)?;
        let copied =
            std::io::copy(&mut from.take(room.saturating_add(1)), &mut file).map_err(damaged)?;
        self.written = self.written.saturating_add(copied);
        if copied > self.limits.entry {
            return Err(UnpackFault::TooLarge(format!(
                "a file is larger than {} bytes",
                self.limits.entry
            )));
        }
        if copied > room {
            return Err(UnpackFault::TooLarge(format!(
                "more than {} bytes in all",
                self.limits.total
            )));
        }
        Ok(())
    }
}

/// Unpacks the archive file `archive` (named `name`, which says the kind) into `into`, which is
/// made, or is an empty folder. Nothing is left in `into` when this fails.
///
/// # Errors
/// See [`UnpackFault`].
pub fn unpack(name: &str, archive: &Path, into: &Path, limits: &Limits) -> Result<(), UnpackFault> {
    let kind = kind(name)?;
    let mut stage_name = into
        .file_name()
        .ok_or_else(|| damaged("no folder name"))?
        .to_owned();
    stage_name.push(".unpacking");
    let stage = into.with_file_name(stage_name);
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).map_err(damaged)?;
    let mut budget = Budget::new(*limits);
    let done = match kind {
        Kind::TarGz => tar_gz(archive, &stage, &mut budget),
        Kind::Zip => zip(archive, &stage, &mut budget),
    }
    .and_then(|()| settle(&stage, into));
    if done.is_err() {
        let _ = std::fs::remove_dir_all(&stage);
    }
    done
}

/// Puts the staged folder in place of `into`, which is absent or empty.
fn settle(stage: &Path, into: &Path) -> Result<(), UnpackFault> {
    match std::fs::remove_dir(into) {
        Ok(()) => {}
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
        Err(why) => return Err(damaged(why)),
    }
    std::fs::rename(stage, into).map_err(damaged)
}

fn tar_gz(archive: &Path, into: &Path, budget: &mut Budget) -> Result<(), UnpackFault> {
    let file = File::open(archive).map_err(damaged)?;
    let mut tarball = tar::Archive::new(GzDecoder::new(BufReader::new(file)));
    for entry in tarball.entries().map_err(damaged)? {
        let mut entry = entry.map_err(damaged)?;
        budget.entry()?;
        let kind = entry.header().entry_type();
        let named = entry.path().map_err(damaged)?.into_owned();
        if !(kind.is_file() || kind.is_dir()) {
            return Err(UnpackFault::Path(named.display().to_string()));
        }
        let rel = inside(&named).ok_or_else(|| UnpackFault::Path(named.display().to_string()))?;
        let target = into.join(rel);
        if kind.is_dir() {
            std::fs::create_dir_all(&target).map_err(damaged)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(damaged)?;
        }
        let mode = entry.header().mode().ok();
        budget.write(&mut entry, &target)?;
        set_mode(&target, mode)?;
    }
    Ok(())
}

fn zip(archive: &Path, into: &Path, budget: &mut Budget) -> Result<(), UnpackFault> {
    let file = File::open(archive).map_err(damaged)?;
    let mut zipped = zip::ZipArchive::new(BufReader::new(file)).map_err(damaged)?;
    for n in 0..zipped.len() {
        budget.entry()?;
        let mut file = zipped.by_index(n).map_err(damaged)?;
        let link = file.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000);
        let shown = file.name().to_owned();
        let rel = file
            .enclosed_name()
            .and_then(|p| inside(&p))
            .filter(|_| !link)
            .ok_or_else(|| UnpackFault::Path(shown))?;
        let target = into.join(rel);
        if file.is_dir() {
            std::fs::create_dir_all(&target).map_err(damaged)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(damaged)?;
        }
        let mode = file.unix_mode();
        budget.write(&mut file, &target)?;
        set_mode(&target, mode)?;
    }
    Ok(())
}

fn set_mode(path: &Path, mode: Option<u32>) -> Result<(), UnpackFault> {
    use std::os::unix::fs::PermissionsExt;
    let Some(mode) = mode else { return Ok(()) };
    let perms = std::fs::Permissions::from_mode(mode & 0o777);
    std::fs::set_permissions(path, perms).map_err(damaged)
}
