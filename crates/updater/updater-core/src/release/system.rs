//! A system file the release asks the privileged helper to place.
//!
//! Links (see [`super::Export`]) cover what the desktop's own session reads. A few things only work as a
//! real file owned by root in a system folder: a udev rule, a polkit rule, a login service, the
//! keyboard daemon's unit. The release lists each such file by its exact destination, the file travels
//! in the bundle under `system/` at the same path, and the helper copies it.
//!
//! The policy is per file, not per folder: a destination is accepted only when [`allowed_system_files`]
//! names it. A folder alone would let a compromised build step add `/etc/polkit-1/rules.d/00-all.rules`
//! to a signed release; with this list that release is refused whole. The cost is deliberate: the helper
//! is installed by hand, not by a release, so a new system file needs a new helper.

use super::error::ReleaseError;
use crate::path::BundlePath;
use serde::Deserialize;
use std::sync::OnceLock;

/// The folder inside the bundle the system files travel in; their destination follows it.
pub const SYSTEM_TREE: &str = "system";

/// Which names are accepted in one folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameRule {
    /// Exactly this name.
    Exact(&'static str),
    /// Any plain name ending in this text (the machine-wide tables, one per daemon).
    Ending(&'static str),
}

impl NameRule {
    fn accepts(self, name: &str) -> bool {
        match self {
            Self::Exact(only) => name == only,
            Self::Ending(tail) => name.len() > tail.len() && name.ends_with(tail),
        }
    }
}

/// One folder and what may be placed directly in it (no sub-folders).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SystemRule {
    /// The folder, absolute, without a trailing slash.
    pub dir: &'static str,
    /// The names accepted in it.
    pub name: NameRule,
}

/// The rules, as `release/system-files.list` of the desktop's packaging tree writes them: one per line,
/// `FOLDER exact|ending NAME`, with `#` comment lines and blank lines. The release tooling
/// (`packaging/release/release.py`) reads the same file, so there is one list.
const LIST: &str = include_str!("../../../../../packaging/release/system-files.list");

/// What is wrong with a line of the list.
#[derive(Debug, PartialEq, Eq)]
struct ListError {
    line: usize,
    text: String,
}

fn parse_rules(list: &'static str) -> Result<Vec<SystemRule>, ListError> {
    let mut rules = Vec::new();
    for (index, line) in list.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let bad = || ListError {
            line: index + 1,
            text: line.to_owned(),
        };
        let mut words = line.split_whitespace();
        let (Some(dir), Some(kind), Some(name), None) =
            (words.next(), words.next(), words.next(), words.next())
        else {
            return Err(bad());
        };
        let name = match kind {
            "exact" => NameRule::Exact(name),
            "ending" => NameRule::Ending(name),
            _ => return Err(bad()),
        };
        if !dir.starts_with('/') || dir.ends_with('/') {
            return Err(bad());
        }
        rules.push(SystemRule { dir, name });
    }
    Ok(rules)
}

/// Every file a release may place, and nothing else.
///
/// # Panics
///
/// When `release/system-files.list` is malformed; a test reads it, so a bad list never ships.
#[must_use]
pub fn allowed_system_files() -> &'static [SystemRule] {
    static RULES: OnceLock<Vec<SystemRule>> = OnceLock::new();
    RULES.get_or_init(|| {
        parse_rules(LIST).unwrap_or_else(|e| {
            panic!(
                "release/system-files.list line {}: expected FOLDER exact|ending NAME, got {:?}",
                e.line, e.text
            )
        })
    })
}

/// A file the release places on the system: `path` is its absolute destination, one of
/// [`allowed_system_files`]; its contents are the bundle file `system/<path>`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct SystemFile {
    /// Where the file goes on the system.
    pub path: String,
}

fn plain_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name.len() <= 255
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'+' | b'@'))
}

/// Whether `path` is a file a release may place: directly in a listed folder, spelled plainly, and
/// accepted by that folder's rule.
#[must_use]
pub fn is_allowed_system_path(path: &str) -> bool {
    let Some((dir, name)) = path.rsplit_once('/') else {
        return false;
    };
    plain_name(name)
        && allowed_system_files()
            .iter()
            .any(|rule| rule.dir == dir && rule.name.accepts(name))
}

impl SystemFile {
    /// The bundle path the contents travel at: `system` followed by the destination.
    ///
    /// # Errors
    ///
    /// [`ReleaseError`] when the destination does not make a valid bundle path.
    pub fn source(&self) -> Result<BundlePath, ReleaseError> {
        Ok(BundlePath::parse(&format!("{SYSTEM_TREE}{}", self.path))?)
    }

    /// Refuses a destination that no rule names, or that is spelled in a way that could reach
    /// another place.
    pub(super) fn check_path(&self) -> Result<(), ReleaseError> {
        if is_allowed_system_path(&self.path) {
            Ok(())
        } else {
            Err(ReleaseError::BadSystemPath(self.path.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_list_reads() {
        let rules = allowed_system_files();
        assert!(rules.contains(&SystemRule {
            dir: "/etc/pam.d",
            name: NameRule::Exact("bardo-shell"),
        }));
        assert!(rules.contains(&SystemRule {
            dir: "/etc/bardo",
            name: NameRule::Ending("-callers.toml"),
        }));
    }

    #[test]
    fn a_malformed_line_names_its_number() {
        for list in [
            "/etc exact",
            "/etc exact a b",
            "/etc maybe a",
            "etc exact a",
            "/etc/ exact a",
        ] {
            let text: &'static str = Box::leak(format!("# comment\n\n{list}\n").into_boxed_str());
            let found = parse_rules(text);
            assert_eq!(
                found,
                Err(ListError {
                    line: 3,
                    text: list.to_owned()
                })
            );
        }
    }
}
