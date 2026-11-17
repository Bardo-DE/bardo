//! The fake of `org.bardo.Storage1` (sync-fakes, which the rig serves to Files) says the words
//! the live service says.

use drive_fs::SkipError;
use sync_fakes::storage::words as fake;
use sync_storage::words;

#[test]
fn the_fake_of_the_service_gives_the_words_of_the_live_one() {
    for (reason, said) in [
        (SkipError::NotOnADrive, fake::NOT_ON_A_DRIVE),
        (SkipError::OnlineOnly, fake::ONLINE_ONLY),
        (SkipError::Offline, fake::OFFLINE),
        (SkipError::Waiting, fake::WAITING),
        (SkipError::Conflict, fake::CONFLICT),
        (SkipError::KeptWithFolder, fake::KEPT_WITH_FOLDER),
        (SkipError::NoDrive, fake::NO_DRIVE),
        (SkipError::NotADocument, fake::NOT_A_DOCUMENT),
        (SkipError::NoFileFormat, fake::NO_FILE_FORMAT),
    ] {
        assert_eq!(words::reason(reason), said, "{reason:?}");
    }
}
