//! The words of `Sync1.Resolve`: how a caller says which side of a stored conflict wins.

use sync_bus::{RESOLVE_KEEP_LOCAL, RESOLVE_KEEP_REMOTE};
use sync_core::Resolution;
use sync_driver::resolve::How;

/// A `how` that is neither word.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "`{0}` is not a way to settle a conflict: use `{RESOLVE_KEEP_LOCAL}` or `{RESOLVE_KEEP_REMOTE}`"
)]
pub struct UnknownHow(pub String);

/// What `how` said (`keep_local`, `keep_remote`).
///
/// # Errors
///
/// Returns [`UnknownHow`] when `text` is neither word.
pub fn parse_how(text: &str) -> Result<How, UnknownHow> {
    match text {
        RESOLVE_KEEP_LOCAL => Ok(How(Resolution::KeepLocal)),
        RESOLVE_KEEP_REMOTE => Ok(How(Resolution::KeepRemote)),
        other => Err(UnknownHow(other.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_words_read_and_anything_else_names_them() {
        const CASES: &[(&str, Option<Resolution>)] = &[
            ("keep_local", Some(Resolution::KeepLocal)),
            ("keep_remote", Some(Resolution::KeepRemote)),
            ("", None),
            ("Keep_Local", None),
            ("keep local", None),
            ("local", None),
        ];
        for (text, want) in CASES {
            let got = parse_how(text);
            assert_eq!(got.as_ref().ok().map(|how| how.0), *want, "{text:?}");
            if let Err(error) = got {
                let said = error.to_string();
                assert!(said.contains("keep_local") && said.contains("keep_remote"));
            }
        }
    }
}
