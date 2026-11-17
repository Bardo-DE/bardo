//! The descriptors that ride with a frame, read into the bytes the bridge maps from.

use inference_bridge::{BridgeError, Frames, MAX_ATTACHMENT};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::OwnedFd;

/// Reads every descriptor from the start (the sender's offset is at the end of what it wrote). A
/// descriptor over [`MAX_ATTACHMENT`] bytes is refused.
///
/// # Errors
/// [`BridgeError::AttachmentRead`] when a descriptor cannot be read, and
/// [`BridgeError::Attachment`] when it is over [`MAX_ATTACHMENT`].
pub fn read_frames(fds: Vec<OwnedFd>) -> Result<Frames, BridgeError> {
    fds.into_iter()
        .map(|fd| {
            let mut file = File::from(fd);
            let size = file.metadata().map_err(BridgeError::AttachmentRead)?.len();
            if size > MAX_ATTACHMENT {
                return Err(BridgeError::Attachment);
            }
            file.seek(SeekFrom::Start(0))
                .map_err(BridgeError::AttachmentRead)?;
            let mut bytes = Vec::with_capacity(usize::try_from(size).unwrap_or(0));
            file.read_to_end(&mut bytes)
                .map_err(BridgeError::AttachmentRead)?;
            Ok(bytes)
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Frames::from_bytes)
}
