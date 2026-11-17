//! The words Files shows for a drive's items and folders, written from the typed reasons drive-fs, drive-fuse and bardo-drive-mount return.
//!
//! syncd serves `org.bardo.Storage1` and the sync settings rows, so the sentences live here and
//! nowhere in those libraries.

use drive_fs::SkipError;
use drive_fuse::MountError;
use drive_fuse::mountd::wire::Code;

/// A drive this computer cannot show as a folder (no way to show files on demand here).
pub const NO_FILES_HERE: &str = "Files can't show this drive on this computer.";

/// Why an action or a change did not go through, as a plain sentence.
#[must_use]
pub fn reason(reason: SkipError) -> &'static str {
    match reason {
        SkipError::NotOnADrive => "This item isn't in a cloud drive.",
        SkipError::OnlineOnly => "This document opens online.",
        SkipError::Offline => "This file will download when you're connected.",
        SkipError::Waiting => "Waiting to upload.",
        SkipError::NoRoom => "There's no room left on this drive.",
        SkipError::MayNotWrite => "You can't change this file.",
        SkipError::InUse => "This file is in use on the drive.",
        SkipError::NameRefused => "The drive doesn't take this name.",
        SkipError::Declined => "The drive couldn't take this change.",
        SkipError::NoRoomHere => "There isn't room on this computer to send this file.",
        SkipError::Conflict => "This file has two versions. Choose one first.",
        SkipError::KeptWithFolder => "This item is kept downloaded with its folder.",
        SkipError::KeptWithDrive => "Everything on this drive is kept downloaded.",
        SkipError::NoRoomForAll => "Not enough space to keep everything downloaded.",
        SkipError::NoDrive => "This drive isn't in Files.",
        SkipError::NotADocument => "Only an online document can be saved as a copy.",
        SkipError::NoFileFormat => "This document can't be saved as a file.",
        SkipError::TooLarge => "This document is too large to save as a copy.",
        SkipError::CopyFailed => "The copy couldn't be made. Try again later.",
    }
}

/// What bardo-drive-mount's refusal says.
#[must_use]
pub fn refusal(code: Code) -> &'static str {
    match code {
        Code::NotHere => NO_FILES_HERE,
        Code::FolderInUse => {
            "The drive's folder in Cloud already has other files in it. Move them, then try again."
        }
        Code::AlreadyShown => "This drive is already shown in Files.",
        Code::NotTakenAway => "Files couldn't stop showing this drive. Try again later.",
    }
}

/// What Files shows for a drive that was not mounted: the helper's word when it refused, else
/// [`NO_FILES_HERE`].
#[must_use]
pub fn unshown(error: &MountError) -> &'static str {
    match error {
        MountError::Refused(code) => refusal(*code),
        _ => NO_FILES_HERE,
    }
}

/// What removing an account kept: `count` files made or changed here that were not uploaded,
/// written out as plain files in the drive's folder (`folder`, as people see it:
/// `~/Cloud/OneDrive`).
#[must_use]
pub fn kept_unsent(count: usize, folder: &str) -> String {
    match count {
        1 => format!("1 file that wasn't uploaded was kept in {folder}."),
        n => format!("{n} files that weren't uploaded were kept in {folder}."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_reason_has_its_own_sentence() {
        let mut seen = std::collections::BTreeSet::new();
        for it in SkipError::ALL {
            let text = reason(it);
            assert!(text.ends_with('.'), "{text}");
            assert!(seen.insert(text), "{text} is said twice");
        }
    }

    #[test]
    fn what_was_kept_is_said_with_its_count_and_folder() {
        assert_eq!(
            kept_unsent(3, "~/Cloud/Drive (Not Uploaded)"),
            "3 files that weren't uploaded were kept in ~/Cloud/Drive (Not Uploaded)."
        );
        assert_eq!(
            kept_unsent(1, "~/Cloud/Drive"),
            "1 file that wasn't uploaded was kept in ~/Cloud/Drive."
        );
    }

    #[test]
    fn a_mount_that_failed_for_another_reason_is_a_drive_files_cannot_show() {
        let error = MountError::NoHelper("fusermount3".to_owned());
        assert_eq!(unshown(&error), NO_FILES_HERE);
        assert_eq!(
            unshown(&MountError::Refused(Code::AlreadyShown)),
            refusal(Code::AlreadyShown)
        );
    }
}
