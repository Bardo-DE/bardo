//! The memory settings' words: the note under a value that waits, why a confirmation failed, and
//! the line under the Memory section.

use detent_model::{Confinement, KeptValue, Trouble};

/// The button that confirms a waiting value.
pub(crate) const CONFIRM: &str = "Confirm";

/// The note under a row whose value waits for the person: what to do, and what memory keeps
/// meanwhile.
pub(crate) fn waiting_note(kept: &KeptValue) -> String {
    let value = match kept {
        KeptValue::Measured(text) | KeptValue::Written(text) => text.clone(),
        KeptValue::Named(label) => format!("\u{201c}{label}\u{201d}"),
    };
    format!("Waits for you to confirm. Until then, memory keeps {value}.")
}

/// The calm line under the row when a confirmation did not go through.
pub(crate) fn trouble_line(trouble: Trouble) -> &'static str {
    match trouble {
        Trouble::NotAllowed => "Memory would not take this confirmation from here.",
        Trouble::Invalid => "Memory could not accept this value, so it keeps its own.",
        Trouble::Failed => "This could not be confirmed. Try again in a moment.",
    }
}

/// The plain line for the person; nothing for `Off` and `Unknown`.
pub(crate) fn confinement_line(confinement: Confinement) -> Option<&'static str> {
    match confinement {
        Confinement::Confined => Some("Memory keeps itself to its own folders"),
        Confinement::Partial => Some(
            "Memory keeps itself to its own folders, though this computer can only enforce part of that",
        ),
        Confinement::NotAvailable => Some("This computer can't keep memory to its own folders"),
        Confinement::Off | Confinement::Unknown => None,
    }
}

/// The words "Confirm {}".
pub(crate) fn confirm(a: impl std::fmt::Display) -> String {
    format!("Confirm {a}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_is_told_only_when_there_is_something_to_say() {
        let cases = [
            (
                Confinement::Confined,
                Some("Memory keeps itself to its own folders"),
            ),
            (
                Confinement::Partial,
                Some(
                    "Memory keeps itself to its own folders, though this computer can only enforce part of that",
                ),
            ),
            (
                Confinement::NotAvailable,
                Some("This computer can't keep memory to its own folders"),
            ),
            (Confinement::Off, None),
            (Confinement::Unknown, None),
        ];
        for (confinement, want) in cases {
            assert_eq!(confinement_line(confinement), want);
        }
    }
}
