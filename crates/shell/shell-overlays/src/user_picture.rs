//! The person's picture as the UI kit's `LockUser` carries it (`UserPicture`): helpers
//! shared by the lock screen and the polkit prompt.

use shell_kit::lock::vocab::LockUser;
use shell_kit::user_picture::picture::UserPicture;

/// The initial an initials face shows, if the picture is one.
#[must_use]
pub fn initial_of(user: &LockUser) -> Option<char> {
    match &user.picture {
        UserPicture::Face(face) => Some(face.initial),
        _ => None,
    }
}
