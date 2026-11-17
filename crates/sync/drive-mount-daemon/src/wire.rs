//! The wire format between syncd and bardo-drive-mount, one request per connection.
//!
//! The socket is a unix stream socket at `$XDG_RUNTIME_DIR/bardo/drive-mount.sock` ([`socket_in`]),
//! mode 0600 in a 0700 directory. The client connects, writes one request and shuts its writing
//! half; the helper reads to the end, answers with one reply and closes.
//!
//! A message is a list of UTF-8 fields, each ended by a NUL byte (a folder name can hold any
//! character but NUL and `/`, so NUL is a safe separator):
//!
//! | message | fields |
//! |---|---|
//! | mount request | `1`, `mount`, the folder name, the access (`ro` or `rw`) |
//! | unmount request | `1`, `unmount`, the folder name |
//! | reply, done | `ok` (a mount's reply carries the open FUSE connection as one `SCM_RIGHTS` fd) |
//! | reply, refused | `no`, the [`Code`] of the refusal (its [`Code::slug`]) |
//!
//! `1` is the format's version; a helper refuses a version it does not know. A request is at
//! most [`MAX_MESSAGE`] bytes, a reply too. The folder name is only a name ([`FolderName`]): the
//! helper puts it under its own `~/Cloud`, so no request can name a path elsewhere. The access is
//! the one choice a caller has about the mount; the helper always adds `nosuid`, `nodev`,
//! `noatime` and `fsname=bardo,subtype=bardo` (so the mount lists as `fuse.bardo`,
//! [`FSTYPE`]).

use rustix::net::{
    RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, SendAncillaryBuffer,
    SendAncillaryMessage, SendFlags,
};
use std::io::{self, IoSlice, IoSliceMut};
use std::mem::MaybeUninit;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::path::{Path, PathBuf};

/// The format's version, the first field of every request.
pub const VERSION: &str = "1";
/// The most bytes a request or a reply may be.
pub const MAX_MESSAGE: usize = 1024;
/// The file system type a drive's folder lists as in `/proc/self/mountinfo`.
pub const FSTYPE: &str = "fuse.bardo";

/// Where the helper listens, under the runtime directory `$XDG_RUNTIME_DIR`.
#[must_use]
pub fn socket_in(runtime_dir: &Path) -> PathBuf {
    runtime_dir.join("bardo/drive-mount.sock")
}

/// A drive's folder name in `~/Cloud`: one path component, with no `/` and no NUL, not `.` or
/// `..` nor any other name with a leading dot, not empty and at most 255 bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderName(String);

/// Why a folder name was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadNameError {
    /// It is empty.
    Empty,
    /// It is longer than 255 bytes.
    TooLong,
    /// It holds `/` or NUL, or starts with a dot (so is not `.` or `..`).
    NotOneName,
}

impl std::fmt::Display for BadNameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            BadNameError::Empty => "the folder name is empty",
            BadNameError::TooLong => "the folder name is longer than 255 bytes",
            BadNameError::NotOneName => "the folder name holds '/' or NUL, or starts with a dot",
        })
    }
}

impl std::error::Error for BadNameError {}

impl FolderName {
    /// Checks a folder name.
    ///
    /// # Errors
    /// What is wrong with it.
    pub fn parse(name: &str) -> Result<Self, BadNameError> {
        if name.is_empty() {
            return Err(BadNameError::Empty);
        }
        if name.len() > 255 {
            return Err(BadNameError::TooLong);
        }
        if name.contains(['/', '\0']) || name.starts_with('.') {
            return Err(BadNameError::NotOneName);
        }
        Ok(Self(name.to_owned()))
    }

    /// The name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FolderName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What the caller may do in the folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Read only: every write is refused.
    ReadOnly,
    /// Read and write: what is changed there is sent to the drive.
    ReadWrite,
}

impl Access {
    /// Its field on the wire.
    #[must_use]
    pub fn field(self) -> &'static str {
        match self {
            Access::ReadOnly => "ro",
            Access::ReadWrite => "rw",
        }
    }

    fn from_field(field: &str) -> Option<Self> {
        match field {
            "ro" => Some(Access::ReadOnly),
            "rw" => Some(Access::ReadWrite),
            _ => None,
        }
    }

    /// The options `fusermount3 -o` is given for it: the access, then what every drive's folder
    /// has.
    #[must_use]
    pub fn options(self) -> String {
        format!(
            "{},nosuid,nodev,noatime,fsname=bardo,subtype=bardo",
            self.field()
        )
    }
}

/// A request to the helper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Show a drive at `~/Cloud/<name>`: answered with the open FUSE connection.
    Mount {
        /// The folder.
        name: FolderName,
        /// What the caller may do there.
        access: Access,
    },
    /// Stop showing the drive at `~/Cloud/<name>`.
    Unmount {
        /// The folder.
        name: FolderName,
    },
}

/// Why a message could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BadMessageError {
    /// It is longer than [`MAX_MESSAGE`].
    TooLong,
    /// It is not UTF-8 fields each ended by NUL.
    NotFields,
    /// It is not UTF-8.
    NotText(std::str::Utf8Error),
    /// A version this side does not know.
    Version(String),
    /// Not a request or reply of this format.
    Unknown(String),
    /// The folder name is refused.
    Name(BadNameError),
}

impl std::fmt::Display for BadMessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BadMessageError::TooLong => write!(f, "the message is longer than {MAX_MESSAGE} bytes"),
            BadMessageError::NotFields => f.write_str("the message is not NUL-ended UTF-8 fields"),
            BadMessageError::NotText(_) => f.write_str("the message is not UTF-8 text"),
            BadMessageError::Version(v) => write!(f, "version {v:?} is not known"),
            BadMessageError::Unknown(what) => write!(f, "{what:?} is not a message of this format"),
            BadMessageError::Name(why) => why.fmt(f),
        }
    }
}

impl std::error::Error for BadMessageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BadMessageError::NotText(why) => Some(why),
            BadMessageError::TooLong
            | BadMessageError::Name(_)
            | BadMessageError::NotFields
            | BadMessageError::Version(_)
            | BadMessageError::Unknown(_) => None,
        }
    }
}

fn encode(fields: &[&str]) -> Vec<u8> {
    let mut out = Vec::new();
    for field in fields {
        out.extend_from_slice(field.as_bytes());
        out.push(0);
    }
    out
}

fn fields(bytes: &[u8]) -> Result<Vec<&str>, BadMessageError> {
    if bytes.len() > MAX_MESSAGE {
        return Err(BadMessageError::TooLong);
    }
    let text = std::str::from_utf8(bytes).map_err(BadMessageError::NotText)?;
    let body = text.strip_suffix('\0').ok_or(BadMessageError::NotFields)?;
    Ok(body.split('\0').collect())
}

impl Request {
    /// The request as bytes.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Request::Mount { name, access } => {
                encode(&[VERSION, "mount", name.as_str(), access.field()])
            }
            Request::Unmount { name } => encode(&[VERSION, "unmount", name.as_str()]),
        }
    }

    /// Reads a request.
    ///
    /// # Errors
    /// The bytes are not a request of this format, or its folder name is refused.
    pub fn decode(bytes: &[u8]) -> Result<Self, BadMessageError> {
        let fields = fields(bytes)?;
        match fields.as_slice() {
            [version, ..] if *version != VERSION => {
                Err(BadMessageError::Version(version.to_string()))
            }
            [_, "mount", name, access] => Ok(Request::Mount {
                name: FolderName::parse(name).map_err(BadMessageError::Name)?,
                access: Access::from_field(access)
                    .ok_or_else(|| BadMessageError::Unknown(format!("access {access}")))?,
            }),
            [_, "unmount", name] => Ok(Request::Unmount {
                name: FolderName::parse(name).map_err(BadMessageError::Name)?,
            }),
            [_, verb, ..] => Err(BadMessageError::Unknown(verb.to_string())),
            _ => Err(BadMessageError::NotFields),
        }
    }
}

/// Why the helper refused, as a code: the words a person reads for it are the caller's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// The drive cannot be shown here: no way to, a request out of the format, a caller not this
    /// person, or a folder that is not a plain folder in Cloud.
    NotHere,
    /// The drive's folder already holds files of its own, or another folder is shown there.
    FolderInUse,
    /// The drive is shown there already.
    AlreadyShown,
    /// The drive's folder could not be taken away.
    NotTakenAway,
}

impl Code {
    /// The code as it travels.
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            Code::NotHere => "not_here",
            Code::FolderInUse => "folder_in_use",
            Code::AlreadyShown => "already_shown",
            Code::NotTakenAway => "not_taken_away",
        }
    }

    /// The code a word stands for.
    #[must_use]
    pub fn from_slug(slug: &str) -> Option<Self> {
        [
            Code::NotHere,
            Code::FolderInUse,
            Code::AlreadyShown,
            Code::NotTakenAway,
        ]
        .into_iter()
        .find(|code| code.slug() == slug)
    }
}

impl std::fmt::Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Code::NotHere => "the drive cannot be shown here",
            Code::FolderInUse => "the folder is in use",
            Code::AlreadyShown => "the drive is shown there already",
            Code::NotTakenAway => "the folder could not be taken away",
        })
    }
}

/// The helper's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reply {
    /// Done; a mount's carries the connection's fd beside it.
    Done,
    /// Refused, for this reason.
    Refused(Code),
}

impl Reply {
    /// The reply as bytes.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Reply::Done => encode(&["ok"]),
            Reply::Refused(code) => encode(&["no", code.slug()]),
        }
    }

    /// Reads a reply.
    ///
    /// # Errors
    /// The bytes are not a reply of this format.
    pub fn decode(bytes: &[u8]) -> Result<Self, BadMessageError> {
        match fields(bytes)?.as_slice() {
            ["ok"] => Ok(Reply::Done),
            ["no", slug] => Code::from_slug(slug)
                .map(Reply::Refused)
                .ok_or_else(|| BadMessageError::Unknown((*slug).to_owned())),
            [what, ..] => Err(BadMessageError::Unknown(what.to_string())),
            [] => Err(BadMessageError::NotFields),
        }
    }
}

/// Sends `bytes` on `socket` with `fd`, when given, beside them (`SCM_RIGHTS`).
///
/// # Errors
/// The socket's.
pub fn send(socket: impl AsFd, bytes: &[u8], fd: Option<BorrowedFd<'_>>) -> io::Result<()> {
    let fds: Vec<BorrowedFd<'_>> = fd.into_iter().collect();
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(1))];
    let mut control = SendAncillaryBuffer::new(&mut space);
    if !fds.is_empty() && !control.push(SendAncillaryMessage::ScmRights(&fds)) {
        return Err(io::Error::other("no room for the fd"));
    }
    let mut sent = 0;
    while sent < bytes.len() {
        let iov = [IoSlice::new(&bytes[sent..])];
        // The fd goes with the first piece only.
        let n = match sent {
            0 => rustix::net::sendmsg(&socket, &iov, &mut control, SendFlags::NOSIGNAL)?,
            _ => rustix::net::send(&socket, &bytes[sent..], SendFlags::NOSIGNAL)?,
        };
        sent += n;
    }
    Ok(())
}

/// Reads `socket` to its end, at most [`MAX_MESSAGE`] bytes, and the first fd sent beside them.
///
/// # Errors
/// The socket's, or a message longer than [`MAX_MESSAGE`].
pub fn receive(socket: impl AsFd) -> io::Result<(Vec<u8>, Option<OwnedFd>)> {
    let mut bytes = Vec::new();
    let mut fd = None;
    let mut buf = [0u8; MAX_MESSAGE + 1];
    loop {
        let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(1))];
        let mut control = RecvAncillaryBuffer::new(&mut space);
        let mut iov = [IoSliceMut::new(&mut buf)];
        let got = rustix::net::recvmsg(&socket, &mut iov, &mut control, RecvFlags::CMSG_CLOEXEC)?;
        for message in control.drain() {
            if let RecvAncillaryMessage::ScmRights(fds) = message {
                for received in fds {
                    // Any fd past the first is closed here, as it is dropped.
                    fd.get_or_insert(received);
                }
            }
        }
        if got.bytes == 0 {
            return Ok((bytes, fd));
        }
        bytes.extend_from_slice(&buf[..got.bytes]);
        if bytes.len() > MAX_MESSAGE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                BadMessageError::TooLong,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_name_is_one_name_and_nothing_else() {
        for good in [
            "OneDrive",
            "Google Drive",
            "OneDrive (Work)",
            "a.b",
            "a..b",
            "日本",
        ] {
            assert_eq!(FolderName::parse(good).expect(good).as_str(), good);
        }
        assert_eq!(FolderName::parse(""), Err(BadNameError::Empty));
        assert_eq!(
            FolderName::parse(&"x".repeat(256)),
            Err(BadNameError::TooLong)
        );
        FolderName::parse(&"x".repeat(255)).unwrap();
        for bad in ["a/b", "/abs", "..", ".", ".hidden", "nul\0", "../Documents"] {
            assert_eq!(
                FolderName::parse(bad),
                Err(BadNameError::NotOneName),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn requests_and_replies_read_back_as_they_were_written() {
        let name = FolderName::parse("Google Drive").expect("name");
        let mount = Request::Mount {
            name: name.clone(),
            access: Access::ReadOnly,
        };
        assert_eq!(mount.encode(), b"1\0mount\0Google Drive\0ro\0");
        assert_eq!(Request::decode(&mount.encode()), Ok(mount));
        let unmount = Request::Unmount { name };
        assert_eq!(unmount.encode(), b"1\0unmount\0Google Drive\0");
        assert_eq!(Request::decode(&unmount.encode()), Ok(unmount));
        for reply in [Reply::Done, Reply::Refused(Code::NotHere)] {
            assert_eq!(Reply::decode(&reply.encode()), Ok(reply));
        }
        assert_eq!(
            Access::ReadOnly.options(),
            "ro,nosuid,nodev,noatime,fsname=bardo,subtype=bardo"
        );
        let writable = Request::Mount {
            name: FolderName::parse("OneDrive").expect("name"),
            access: Access::ReadWrite,
        };
        assert_eq!(writable.encode(), b"1\0mount\0OneDrive\0rw\0");
        assert_eq!(Request::decode(&writable.encode()), Ok(writable));
        assert_eq!(
            Access::ReadWrite.options(),
            "rw,nosuid,nodev,noatime,fsname=bardo,subtype=bardo"
        );
    }

    #[test]
    fn a_request_out_of_the_format_is_refused() {
        let invalid = b"1\0mount\0\xff\0ro\0".to_vec();
        let not_text = std::str::from_utf8(&invalid).expect_err("not text");
        let cases: [(&[u8], BadMessageError); 7] = [
            (
                b"2\0mount\0OneDrive\0ro\0",
                BadMessageError::Version("2".into()),
            ),
            (
                b"1\0mount\0OneDrive\0rwx\0",
                BadMessageError::Unknown("access rwx".into()),
            ),
            (
                b"1\0format\0OneDrive\0",
                BadMessageError::Unknown("format".into()),
            ),
            (
                b"1\0mount\0..\0ro\0",
                BadMessageError::Name(BadNameError::NotOneName),
            ),
            (b"1\0unmount\0", BadMessageError::Unknown("unmount".into())),
            (b"1\0mount\0OneDrive\0ro", BadMessageError::NotFields),
            (b"1\0mount\0\xff\0ro\0", BadMessageError::NotText(not_text)),
        ];
        for (bytes, why) in cases {
            assert_eq!(Request::decode(bytes), Err(why), "{bytes:?}");
        }
        assert_eq!(
            Request::decode(&vec![b'a'; MAX_MESSAGE + 1]),
            Err(BadMessageError::TooLong)
        );
    }

    #[test]
    fn a_refusal_travels_as_its_code() {
        for code in [
            Code::NotHere,
            Code::FolderInUse,
            Code::AlreadyShown,
            Code::NotTakenAway,
        ] {
            let bytes = Reply::Refused(code).encode();
            assert_eq!(Reply::decode(&bytes), Ok(Reply::Refused(code)));
        }
        Reply::decode(b"no\0There is no room.\0").unwrap_err();
    }
}
