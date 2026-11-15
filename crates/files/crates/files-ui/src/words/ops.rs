//! What is said while files change: the toast after a move to the Trash, the progress sheet and
//! the questions, in Finder's words. Pure.

use files_core::{Conflict, Bring, OpKind, Summary};
use files_model::ExtensionChange;

/// `“name”` in typographic quotes.
fn quoted(summary: &Summary) -> String {
    let name = summary.first.as_ref().map(|n| n.display().into_owned());
    format!("\u{201c}{}\u{201d}", name.unwrap_or_default())
}

/// The toast after a move to the Trash: `Moved “x” to the Trash`, or the count for several.
pub fn announce_words(summary: &Summary) -> String {
    match summary.count {
        1 => format!("Moved {} to the Trash", quoted(summary)),
        count => format!("Moved {count} items to the Trash"),
    }
}

/// What the progress sheet says is going on: `Copying “x”`, `Moving 3 items to the Trash`.
pub fn busy_words(summary: &Summary) -> String {
    let (doing, after) = match summary.kind {
        OpKind::Copy => ("Copying", ""),
        OpKind::Move => ("Moving", ""),
        OpKind::NewFolder => ("Making a folder", ""),
        OpKind::Rename => ("Renaming", ""),
        OpKind::Trash => ("Moving", " to the Trash"),
        OpKind::PutBack => ("Putting back", ""),
        OpKind::Erase => ("Erasing", ""),
    };
    match summary.count {
        0 => format!("{doing}{after}"),
        1 => format!("{doing} {}{after}", quoted(summary)),
        count => format!("{doing} {count} items{after}"),
    }
}

/// The question about a taken name: its title, and the message under it.
pub fn conflict_words(conflict: &Conflict) -> (String, &'static str) {
    let title = format!(
        "An item named \u{201c}{}\u{201d} already exists in this location.",
        conflict.item
    );
    let message = match conflict.kind {
        Bring::Copy => "Do you want to replace it with the one you're copying?",
        Bring::Move => "Do you want to replace it with the one you're moving?",
    };
    (title, message)
}

/// The title of the question before erasing `count` items of the Trash for good, the first being
/// `first`; `None` for the whole Trash.
pub fn erase_words(count: Option<(usize, String)>) -> String {
    match count {
        None => "Are you sure you want to permanently erase the items in the Trash?".to_owned(),
        Some((1, name)) => {
            format!("Are you sure you want to permanently erase \u{201c}{name}\u{201d}?")
        }
        Some((count, _)) => {
            format!("Are you sure you want to permanently erase these {count} items?")
        }
    }
}

/// The message under any question about erasing.
pub const CANNOT_UNDO: &str = "You can't undo this action.";

/// What the question before changing a file's extension says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionWords {
    /// The question.
    pub title: String,
    /// The line under it.
    pub message: &'static str,
    /// The button that goes ahead with the new name.
    pub go_ahead: String,
    /// The button that keeps the old extension.
    pub keep: String,
}

/// The question before a rename that changes, adds or takes away a file's extension.
pub fn extension_words(change: &ExtensionChange) -> ExtensionWords {
    let quoted = |extension: &str| format!("\u{201c}.{extension}\u{201d}");
    let message = "If you make this change, your document may open in a different application.";
    match (change.old_extension(), change.new_extension()) {
        (Some(old), Some(new)) => ExtensionWords {
            title: format!(
                "Are you sure you want to change the extension from {} to {}?",
                quoted(&old),
                quoted(&new)
            ),
            message,
            go_ahead: format!("Use .{new}"),
            keep: format!("Keep .{old}"),
        },
        (None, Some(new)) => ExtensionWords {
            title: format!(
                "Are you sure you want to add the extension {} to the end of the name?",
                quoted(&new)
            ),
            message,
            go_ahead: format!("Add .{new}"),
            keep: "Don't Add".to_owned(),
        },
        (Some(old), None) => ExtensionWords {
            title: format!(
                "Are you sure you want to remove the extension {}?",
                quoted(&old)
            ),
            message,
            go_ahead: "Remove".to_owned(),
            keep: format!("Keep .{old}"),
        },
        // Neither name has one: nothing to ask, but the words stay plain.
        (None, None) => ExtensionWords {
            title: "Are you sure you want to rename this item?".to_owned(),
            message,
            go_ahead: "Rename".to_owned(),
            keep: "Cancel".to_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use files_core::{FileName, FolderPath};

    use super::*;

    fn summary(kind: OpKind, count: usize, first: &str) -> Summary {
        let first = (count > 0).then(|| FileName::new(first).unwrap());
        Summary::new(kind, count, first)
    }

    #[test]
    fn a_move_to_the_trash_is_announced_as_finder_does() {
        use OpKind::Trash;
        assert_eq!(
            announce_words(&summary(Trash, 1, "report.pdf")),
            "Moved \u{201c}report.pdf\u{201d} to the Trash"
        );
        assert_eq!(
            announce_words(&summary(Trash, 3, "report.pdf")),
            "Moved 3 items to the Trash"
        );
    }

    #[test]
    fn work_in_progress_says_what_it_is() {
        use OpKind::{Copy, Erase, Trash};
        let table = [
            (summary(Copy, 1, "a.txt"), "Copying \u{201c}a.txt\u{201d}"),
            (summary(Copy, 12, "a.txt"), "Copying 12 items"),
            (summary(Trash, 2, "a"), "Moving 2 items to the Trash"),
            (summary(Erase, 0, ""), "Erasing"),
        ];
        for (given, want) in table {
            assert_eq!(busy_words(&given), want);
        }
    }

    #[test]
    fn a_taken_name_is_asked_in_finders_words() {
            let here = FolderPath::new("/d").unwrap();
        let ask = |kind| Conflict::new(FileName::new("a.txt").unwrap(), here.clone(), kind, 0);
        let (title, message) = conflict_words(&ask(Bring::Move));
        assert_eq!(
            title,
            "An item named \u{201c}a.txt\u{201d} already exists in this location."
        );
        assert_eq!(
            message,
            "Do you want to replace it with the one you're moving?"
        );
        let (_, copying) = conflict_words(&ask(Bring::Copy));
        assert_eq!(
            copying,
            "Do you want to replace it with the one you're copying?"
        );
    }

    #[test]
    fn erasing_asks_about_what_it_erases() {
        assert_eq!(
            erase_words(None),
            "Are you sure you want to permanently erase the items in the Trash?"
        );
        assert_eq!(
            erase_words(Some((1, "a.txt".to_owned()))),
            "Are you sure you want to permanently erase \u{201c}a.txt\u{201d}?"
        );
        assert_eq!(
            erase_words(Some((4, "a.txt".to_owned()))),
            "Are you sure you want to permanently erase these 4 items?"
        );
    }

    #[test]
    fn a_changed_extension_is_asked_about_in_finders_words() {
        let name = |text: &str| FileName::new(text).unwrap();
        let change = |from: &str, to: &str| {
            let folder = FolderPath::new("/home/p").unwrap();
            ExtensionChange::new(folder, name(from), name(to), name(from))
        };
        // (label, from, to, the question, the button that goes ahead, the button that keeps)
        let table = [
            (
                "changed",
                "a.txt",
                "a.md",
                "Are you sure you want to change the extension from \u{201c}.txt\u{201d} to \u{201c}.md\u{201d}?",
                "Use .md",
                "Keep .txt",
            ),
            (
                "added",
                "a",
                "a.md",
                "Are you sure you want to add the extension \u{201c}.md\u{201d} to the end of the name?",
                "Add .md",
                "Don't Add",
            ),
            (
                "removed",
                "a.txt",
                "a",
                "Are you sure you want to remove the extension \u{201c}.txt\u{201d}?",
                "Remove",
                "Keep .txt",
            ),
        ];
        for (label, from, to, title, go_ahead, keep) in table {
            let words = extension_words(&change(from, to));
            assert_eq!(words.title, title, "{label}");
            assert_eq!(words.go_ahead, go_ahead, "{label}");
            assert_eq!(words.keep, keep, "{label}");
        }
    }
}
