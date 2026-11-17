//! Turning what the system says into `files_core::OpError`, the one error of a change.

use std::io;

use files_core::OpError;
use files_trash::TrashError;

/// The failure an I/O error stands for; for `map_err`.
#[expect(
    clippy::needless_pass_by_value,
    reason = "it is the argument of `map_err`, which hands the error over by value"
)]
pub(crate) fn io(err: io::Error) -> OpError {
    OpError::from_io(&err)
}

/// The failure a Trash error stands for; for `map_err`.
pub(crate) fn trash(err: TrashError) -> OpError {
    match err {
        TrashError::NotFound => OpError::NotFound,
        TrashError::NoPermission => OpError::NoPermission,
        TrashError::Occupied => OpError::Occupied,
        TrashError::Other => OpError::TrashUnavailable,
    }
}
