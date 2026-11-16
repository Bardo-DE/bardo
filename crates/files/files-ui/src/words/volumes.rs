//! What is said when a disk would not eject or open, in Finder's words. Pure.

use files_core::{EjectFailure, MountFailure, ProgramName, VolumeName, VolumeTrouble};

/// The sentence for a disk that did not eject or open.
///
/// A busy disk says so as Finder does, and names the programs when they could be found.
pub fn volume_trouble_words(name: &VolumeName, trouble: &VolumeTrouble) -> String {
    let disk = format!("The disk \u{201c}{name}\u{201d}");
    match trouble {
        VolumeTrouble::NotEjected(failure) => match failure {
            EjectFailure::InUse(programs) => {
                let busy = format!(
                    "{disk} wasn\u{2019}t ejected because one or more programs may be using it"
                );
                if programs.is_empty() {
                    format!("{busy}.")
                } else {
                    let names: Vec<&str> = programs.iter().map(|p| p.as_str()).collect();
                    format!("{busy}: {}.", names.join(", "))
                }
            }
            EjectFailure::NotAllowed => {
                format!("{disk} wasn\u{2019}t ejected because you don\u{2019}t have permission.")
            }
            EjectFailure::Gone => format!("{disk} is not there any more."),
            EjectFailure::Other => format!("{disk} wasn\u{2019}t ejected."),
        },
        VolumeTrouble::NotOpened(failure) => match failure {
            MountFailure::NotAllowed => {
                format!("{disk} wasn\u{2019}t opened because you don\u{2019}t have permission.")
            }
            MountFailure::Gone => format!("{disk} is not there any more."),
            MountFailure::Other => format!("{disk} couldn\u{2019}t be opened."),
        },
    }
}

/// Finder's question for a disk that is in use: the title, and the programs found as the
/// informative text (a line about forcing when none could be found).
pub fn force_eject_words(name: &VolumeName, programs: &[ProgramName]) -> (String, String) {
    let title = format!(
        "The disk \u{201c}{name}\u{201d} wasn\u{2019}t ejected because one or more programs may be \
         using it."
    );
    let message = if programs.is_empty() {
        "To eject the disk immediately, click Force Eject.".to_owned()
    } else {
        let names: Vec<&str> = programs.iter().map(|p| p.as_str()).collect();
        format!("Programs using it: {}.", names.join(", "))
    };
    (title, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_disk_that_would_not_eject_or_open_says_so_as_finder_does() {
        let name = VolumeName::new("STICK");
        let busy_by = |names: &[&str]| {
            VolumeTrouble::NotEjected(EjectFailure::InUse(
                names.iter().map(|n| ProgramName::new(*n)).collect(),
            ))
        };
        let cases = [
            (
                "busy, programs unknown",
                busy_by(&[]),
                "The disk \u{201c}STICK\u{201d} wasn\u{2019}t ejected because one or more programs may be using it.",
            ),
            (
                "busy, programs known",
                busy_by(&["vlc", "Files"]),
                "The disk \u{201c}STICK\u{201d} wasn\u{2019}t ejected because one or more programs may be using it: vlc, Files.",
            ),
            (
                "not allowed to eject",
                VolumeTrouble::NotEjected(EjectFailure::NotAllowed),
                "The disk \u{201c}STICK\u{201d} wasn\u{2019}t ejected because you don\u{2019}t have permission.",
            ),
            (
                "ejected badly",
                VolumeTrouble::NotEjected(EjectFailure::Other),
                "The disk \u{201c}STICK\u{201d} wasn\u{2019}t ejected.",
            ),
            (
                "not opened",
                VolumeTrouble::NotOpened(MountFailure::Other),
                "The disk \u{201c}STICK\u{201d} couldn\u{2019}t be opened.",
            ),
        ];
        for (label, trouble, want) in cases {
            assert_eq!(volume_trouble_words(&name, &trouble), want, "{label}");
        }
    }

    #[test]
    fn the_force_eject_question_names_the_disk_and_the_programs_using_it() {
        let name = VolumeName::new("STICK");
        let programs = |names: &[&str]| -> Vec<ProgramName> {
            names.iter().map(|n| ProgramName::new(*n)).collect()
        };
        let title = "The disk \u{201c}STICK\u{201d} wasn\u{2019}t ejected because one or more programs may be using it.";
        let cases = [
            (
                "programs found",
                programs(&["vlc", "bash"]),
                "Programs using it: vlc, bash.",
            ),
            ("one program", programs(&["vlc"]), "Programs using it: vlc."),
            (
                "none found",
                programs(&[]),
                "To eject the disk immediately, click Force Eject.",
            ),
        ];
        for (label, found, message) in cases {
            let (got_title, got_message) = force_eject_words(&name, &found);
            assert_eq!(
                (got_title.as_str(), got_message.as_str()),
                (title, message),
                "{label}"
            );
        }
    }
}
