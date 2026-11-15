//! Copying and moving items to a folder, and settling the names that are taken.

use std::fs;
use std::ops::ControlFlow;
use std::os::unix::fs::MetadataExt;

use files_core::{
    Answer, Bring, ByteSize, Conflict, FileName, FolderPath, ItemRef, OpFailure, Progress,
    Resolution, Split, Transfer, keep_both,
};

use super::tree::{bytes_under, remove_tree};
use super::{Run, Step};
use crate::error;
use crate::journal::Inverse;

/// What was asked of the items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum How {
    Copy,
    Move,
    /// Moves when the folder is on the items' disk, copies otherwise.
    Drag,
}

/// Where an item goes, once the name is settled.
enum Place {
    At(FileName),
    Skip,
    Stop,
}

impl Run<'_> {
    pub(super) fn transfer(&mut self, how: How, transfer: &Transfer) -> Step {
        let destination = &transfer.destination;
        let folder = fs::metadata(destination.as_path()).map_err(error::io)?;
        if !folder.is_dir() {
            return Err(OpFailure::NotFound);
        }
        let bytes: u64 = transfer
            .sources
            .iter()
            .map(|source| bytes_under(&source.path()))
            .sum();
        self.progress = Progress::start(transfer.sources.len(), ByteSize(bytes));
        self.emit();
        self.touch_folder(destination);
        let count = transfer.sources.len();
        for (index, source) in transfer.sources.iter().enumerate() {
            if self.host.flow() == ControlFlow::Break(()) {
                return Ok(ControlFlow::Break(()));
            }
            let rest = count - index - 1;
            if self.one(how, source, destination, folder.dev(), rest)? == ControlFlow::Break(()) {
                return Ok(ControlFlow::Break(()));
            }
            self.progress = self.progress.finished_one();
            self.emit();
        }
        Ok(ControlFlow::Continue(()))
    }

    fn one(
        &mut self,
        how: How,
        source: &ItemRef,
        destination: &FolderPath,
        disk: u64,
        rest: usize,
    ) -> Step {
        let from = source.path();
        let meta = fs::symlink_metadata(&from).map_err(error::io)?;
        let mode = match how {
            How::Copy => Bring::Copy,
            How::Move => Bring::Move,
            How::Drag if meta.dev() == disk => Bring::Move,
            How::Drag => Bring::Copy,
        };
        if meta.is_dir() && destination.as_path().starts_with(&from) {
            return Err(OpFailure::IntoItself);
        }
        let here = source.folder == *destination;
        if here && mode == Bring::Move {
            return Ok(ControlFlow::Continue(()));
        }
        let split = if meta.is_dir() {
            Split::Whole
        } else {
            Split::Extension
        };
        let name = match self.settle(source, destination, mode, split, here, rest)? {
            Place::At(name) => name,
            Place::Skip => return Ok(ControlFlow::Continue(())),
            Place::Stop => return Ok(ControlFlow::Break(())),
        };
        let target = ItemRef::new(destination.clone(), name);
        match mode {
            Bring::Copy => self.copy_one(&from, &target),
            Bring::Move => self.move_one(source, &target),
        }
    }

    fn copy_one(&mut self, from: &std::path::Path, target: &ItemRef) -> Step {
        match self.copy_tree(from, &target.path()) {
            Ok(ControlFlow::Continue(())) => {
                self.steps.push(Inverse::Trash(vec![target.clone()]));
                Ok(ControlFlow::Continue(()))
            }
            Ok(ControlFlow::Break(())) => {
                let _ = remove_tree(&target.path());
                Ok(ControlFlow::Break(()))
            }
            Err(failure) => {
                let _ = remove_tree(&target.path());
                Err(failure)
            }
        }
    }

    fn move_one(&mut self, source: &ItemRef, target: &ItemRef) -> Step {
        if self.move_path(&source.path(), &target.path())? == ControlFlow::Break(()) {
            return Ok(ControlFlow::Break(()));
        }
        self.steps.push(Inverse::MoveBack {
            now: target.clone(),
            back: source.clone(),
        });
        self.touch_folder(&source.folder);
        Ok(ControlFlow::Continue(()))
    }

    /// The name `source` takes in `destination`: its own when free; when taken, what the person
    /// answers (or, for a copy into its own folder, the next free `name 2`, with no question).
    fn settle(
        &mut self,
        source: &ItemRef,
        destination: &FolderPath,
        mode: Bring,
        split: Split,
        here: bool,
        rest: usize,
    ) -> Result<Place, OpFailure> {
        let name = &source.name;
        let taken = |candidate: &FileName| {
            fs::symlink_metadata(destination.join(candidate).as_path()).is_ok()
        };
        let another = || {
            keep_both(name, split, taken)
                .map(Place::At)
                .map_err(|_| OpFailure::NameTooLong)
        };
        if !taken(name) {
            return Ok(Place::At(name.clone()));
        }
        if here {
            return another();
        }
        let conflict = Conflict::new(name.clone(), destination.clone(), mode, rest);
        let resolution = match self.sticky {
            Some(resolution) => resolution,
            None => self.host.ask(&conflict),
        };
        if resolution.to_rest {
            self.sticky = Some(Resolution::for_rest(resolution.answer));
        }
        match resolution.answer {
            Answer::KeepBoth => another(),
            Answer::Skip => Ok(Place::Skip),
            Answer::Stop => Ok(Place::Stop),
            Answer::Replace => {
                let existing = ItemRef::new(destination.clone(), name.clone());
                let thrown = self.trash.trash(&[existing]).map_err(error::trash)?;
                self.steps.push(Inverse::PutBack(thrown));
                self.touch_trash();
                Ok(Place::At(name.clone()))
            }
        }
    }
}
