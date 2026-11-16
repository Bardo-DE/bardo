//! What each kind of file offers: its actions, its edits and whether the viewer has a stage for it.
//! The one match on `FormatKind`; `actions_for`, `edits_for` and `has_stage` read from it.

use crate::action::FileAction;
use crate::edit::EditKind;
use crate::kind::{FormatKind, Mime};

/// Everything that varies by kind of file.
#[derive(Debug, Clone, Copy)]
struct KindProfile {
    mime: &'static str,
    actions: &'static [FileAction],
    edits: &'static [EditKind],
    stage: bool,
}

use FileAction::{
    ConvertTo, CopyFile, CopyPath, Duplicate, Export, FlipHorizontal, FlipVertical, MoveToTrash,
    Open, PlayInBackground, PlayInMiniWindow, Print, Rename, RevealInFolder, RevertTo, RotateLeft,
    RotateRight, SaveCopy, Share, Trim,
};

/// A file with nothing to do but handle it.
const PLAIN: &[FileAction] = &[
    Open,
    RevealInFolder,
    CopyFile,
    CopyPath,
    Share,
    Rename,
    Duplicate,
    MoveToTrash,
];
const IMAGE: &[FileAction] = &[
    Open,
    RevealInFolder,
    CopyFile,
    CopyPath,
    Share,
    Rename,
    Duplicate,
    MoveToTrash,
    Print,
    Export,
    ConvertTo,
    SaveCopy,
    RevertTo,
    RotateLeft,
    RotateRight,
    FlipHorizontal,
    FlipVertical,
];
const PDF: &[FileAction] = &[
    Open,
    RevealInFolder,
    CopyFile,
    CopyPath,
    Share,
    Rename,
    Duplicate,
    MoveToTrash,
    Print,
    Export,
    ConvertTo,
    SaveCopy,
    RevertTo,
    RotateLeft,
    RotateRight,
];
const VECTOR: &[FileAction] = &[
    Open,
    RevealInFolder,
    CopyFile,
    CopyPath,
    Share,
    Rename,
    Duplicate,
    MoveToTrash,
    Print,
    Export,
];
const VIDEO: &[FileAction] = &[
    Open,
    RevealInFolder,
    CopyFile,
    CopyPath,
    Share,
    Rename,
    Duplicate,
    MoveToTrash,
    Export,
    SaveCopy,
    RevertTo,
    Trim,
    PlayInMiniWindow,
];
const AUDIO: &[FileAction] = &[
    Open,
    RevealInFolder,
    CopyFile,
    CopyPath,
    Share,
    Rename,
    Duplicate,
    MoveToTrash,
    Export,
    ConvertTo,
    SaveCopy,
    RevertTo,
    Trim,
    PlayInBackground,
];
const TEXT: &[FileAction] = &[
    Open,
    RevealInFolder,
    CopyFile,
    CopyPath,
    Share,
    Rename,
    Duplicate,
    MoveToTrash,
    Print,
    Export,
    ConvertTo,
];
/// A table or a document of nodes, which the viewer edits in place: what a plain file has, and the
/// versions that edit keeps.
const DATA: &[FileAction] = &[
    Open,
    RevealInFolder,
    CopyFile,
    CopyPath,
    Share,
    Rename,
    Duplicate,
    MoveToTrash,
    SaveCopy,
    RevertTo,
];
const FOLDER: &[FileAction] = &[
    Open,
    RevealInFolder,
    CopyFile,
    CopyPath,
    Share,
    Rename,
    Duplicate,
    MoveToTrash,
];

const NO_EDITS: &[EditKind] = &[];
const IMAGE_EDITS: &[EditKind] = &[EditKind::Rotate, EditKind::Flip, EditKind::Adjust];
const MEDIA_EDITS: &[EditKind] = &[EditKind::Trim];
const PDF_EDITS: &[EditKind] = &[
    EditKind::Rotate,
    EditKind::DeletePages,
    EditKind::MovePage,
    EditKind::Markup,
];

const fn profile(
    mime: &'static str,
    actions: &'static [FileAction],
    edits: &'static [EditKind],
    stage: bool,
) -> KindProfile {
    KindProfile {
        mime,
        actions,
        edits,
        stage,
    }
}

fn profile_of(kind: FormatKind) -> KindProfile {
    match kind {
        FormatKind::Pdf => profile("application/pdf", PDF, PDF_EDITS, true),
        FormatKind::Raster => profile("image/png", IMAGE, IMAGE_EDITS, true),
        FormatKind::Vector => profile("image/svg+xml", VECTOR, NO_EDITS, true),
        FormatKind::Video => profile("video/mp4", VIDEO, MEDIA_EDITS, true),
        FormatKind::Audio => profile("audio/mpeg", AUDIO, MEDIA_EDITS, true),
        FormatKind::Markdown => profile("text/markdown", TEXT, NO_EDITS, true),
        FormatKind::Code | FormatKind::PlainText => profile("text/plain", TEXT, NO_EDITS, true),
        FormatKind::Table => profile("text/csv", DATA, NO_EDITS, true),
        FormatKind::Tree => profile("application/json", DATA, NO_EDITS, true),
        FormatKind::Font => profile("font/ttf", PLAIN, NO_EDITS, false),
        FormatKind::Archive => profile("application/zip", PLAIN, NO_EDITS, false),
        FormatKind::Book => profile("application/epub+zip", PLAIN, NO_EDITS, true),
        FormatKind::Office => profile(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            PLAIN,
            NO_EDITS,
            false,
        ),
        FormatKind::Folder => profile("inode/directory", FOLDER, NO_EDITS, false),
        FormatKind::Other => profile("application/octet-stream", PLAIN, NO_EDITS, false),
    }
}

/// The actions a file of `kind` offers, in menu order. The launcher shows those whose `reach`
/// includes it, and the viewer those that include the viewer.
#[must_use]
pub fn actions_for(kind: FormatKind) -> &'static [FileAction] {
    profile_of(kind).actions
}

/// A media type a file of `kind` has, for a row that knows only the kind (a recently viewed
/// file the history names): the commonest type of the kind. `kind_of_mime` of it is `kind`, except
/// that code and plain text share `text/plain`, which names plain text.
#[must_use]
pub fn mime_for(kind: FormatKind) -> Mime {
    Mime::known(profile_of(kind).mime)
}

/// The sorts of edit a file of `kind` can be saved with in place.
#[must_use]
pub fn edits_for(kind: FormatKind) -> &'static [EditKind] {
    profile_of(kind).edits
}

/// Whether the viewer has a full stage for `kind` (else it shows only the light tier). Every kind
/// is answered: a kind without a stage offers facts.
#[must_use]
pub fn has_stage(kind: FormatKind) -> bool {
    profile_of(kind).stage
}
