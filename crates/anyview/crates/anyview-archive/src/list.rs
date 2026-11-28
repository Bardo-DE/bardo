//! Listing an archive: the one entry point, dispatching on how the format is stored.

use crate::container::{Codec, Container, container};
use crate::entry::{Entry, EntryCount, EntryKind, EntryLimit, Listing, Tally};
use crate::error::ArchiveError;
use crate::{sevenz, stream, tar, zip_archive};
use anyview_core::work::Interrupt;
use anyview_core::{ByteLen, Deadline, Input};
use file_types::ArchiveFormat;
use std::io::Cursor;

/// The start of the index of the archive `src`, whose format `format` was sniffed.
///
/// `src` is a path, or any source a host injects. `budget` is the most bytes it reads from the file (the index of a zip, a 7z or a tar's
/// headers) or unpacks from a compressed stream; an index larger than that is
/// [`ArchiveError::OverBudget`] for a zip or a 7z, and a lower-bound count for a tar or a stream.
/// `limit` is how many entries it keeps. Blocking: run it on a worker.
///
/// # Errors
/// [`ArchiveError`] when the archive cannot be opened or read, or is over `budget` (zip, 7z).
pub fn list(
    src: impl Into<Input>,
    format: ArchiveFormat,
    limit: EntryLimit,
    budget: ByteLen,
) -> Result<Listing, ArchiveError> {
    list_until(src, format, limit, budget, &Deadline::NEVER)
}

/// [`list`], ending early when `interrupt` says so (a peek's [`Deadline`], a viewer job's stop).
///
/// The entries read so far come back as a start with a lower-bound count, and a listing that
/// was interrupted before it had any entry is [`ArchiveError::Interrupted`]. It is asked once per
/// entry, and once per chunk while a compressed stream unpacks; the codecs and the zip and 7z
/// crates' own parsing of an index are not reached by it.
///
/// # Errors
/// As [`list`], and [`ArchiveError::Interrupted`] when interrupted before any entry.
pub fn list_until(
    src: impl Into<Input>,
    format: ArchiveFormat,
    limit: EntryLimit,
    budget: ByteLen,
    interrupt: &dyn Interrupt,
) -> Result<Listing, ArchiveError> {
    if budget.0 == 0 {
        return Err(ArchiveError::NoBudget);
    }
    let src = &src.into();
    match container(format) {
        Container::Zip => zip_archive::list(src, format, limit, budget, interrupt),
        Container::Tar => tar::list(src, limit, budget, interrupt),
        Container::Compressed(codec) => compressed(src, format, codec, limit, budget, interrupt),
        Container::SevenZip => sevenz::list(src, limit, budget, interrupt),
    }
}

/// The name of `src` without its last extension: what a compressed stream's one file is called.
pub(crate) fn stem_of(src: &Input) -> Option<String> {
    std::path::Path::new(src.name().as_str())
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
}

/// A compressed stream holds a tar or one file; the first bytes it unpacks to say which.
fn compressed(
    src: &Input,
    format: ArchiveFormat,
    codec: Codec,
    limit: EntryLimit,
    budget: ByteLen,
    interrupt: &dyn Interrupt,
) -> Result<Listing, ArchiveError> {
    let unpacked = stream::unpack(src, format, codec, budget.0, interrupt)?;
    let mut tally = Tally::new(limit);
    let mut archive = ::tar::Archive::new(Cursor::new(&*unpacked.bytes));
    let read = tar::fill(&mut archive, &mut tally, interrupt);
    if tally.is_empty() && unpacked.interrupted {
        // What was unpacked is too little to say whether it is a tar or one file.
        return Err(ArchiveError::Interrupted);
    }
    if tally.is_empty() {
        return Ok(one_file(src, format, &unpacked));
    }
    let complete = matches!(read, Ok(true)) && unpacked.complete;
    Ok(tally.finish(format, complete))
}

fn one_file(src: &Input, format: ArchiveFormat, unpacked: &stream::Unpacked) -> Listing {
    let name = stem_of(src).unwrap_or_default();
    let size = unpacked
        .complete
        .then_some(ByteLen(unpacked.bytes.len() as u64));
    Listing {
        format,
        one_file: true,
        entries: vec![Entry {
            path: name,
            kind: EntryKind::File,
            size,
        }],
        count: EntryCount::Exact(1),
        unpacked: size.unwrap_or(ByteLen(0)),
    }
}
