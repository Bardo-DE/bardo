//! `cargo xtask check error-names`: a type that implements `std::error::Error` is named `...Error`.
//!
//! The scan reads lines, not syntax. It finds `impl ... Error for Name` and a
//! `#[derive(... Error ...)]` (also `thiserror::Error`) with the `struct` or `enum` that follows
//! its attributes. A name that does not end in `Error` is a hit.

use std::fs;

use anyhow::{Context, Result};

use super::{finish, git_lines};
use crate::workspace::Workspace;

/// This file shows the patterns in its tests, so its own text is not read.
const THIS_FILE: &str = "xtask/src/tasks/error_names.rs";

/// The traits a line may name for the standard error trait.
const ERROR_TRAITS: [&str; 5] = [
    "Error",
    "error::Error",
    "std::error::Error",
    "core::error::Error",
    "::std::error::Error",
];

/// The text of the first `<...>` of `text` (which starts with `<`) removed, with nesting.
fn after_generics(text: &str) -> &str {
    let Some(rest) = text.strip_prefix('<') else {
        return text;
    };
    let mut depth = 1_usize;
    for (at, c) in rest.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    return &rest[at + 1..];
                }
            }
            _ => {}
        }
    }
    ""
}

/// The last segment of the path that starts `text`: `a::b::Name<T>` is `Name`.
fn last_segment(text: &str) -> Option<&str> {
    let end = text
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':'))
        .unwrap_or(text.len());
    let name = text[..end].rsplit("::").next()?;
    (!name.is_empty()).then_some(name)
}

/// The type name when `line` is `impl [<..>] <error trait> for <Type> ...`.
fn impl_error_for(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix("impl")?;
    if !rest.starts_with([' ', '<']) {
        return None;
    }
    let rest = after_generics(rest.trim_start()).trim_start();
    let (the_trait, target) = rest.split_once(" for ")?;
    if !ERROR_TRAITS.contains(&the_trait.trim()) {
        return None;
    }
    let target = target.trim_start().trim_start_matches('&').trim_start();
    let target = target
        .strip_prefix('\'')
        .and_then(|lifetime| lifetime.split_once(' '))
        .map_or(target, |(_, after)| after.trim_start());
    last_segment(target)
}

/// Whether the derive list `derive` (the text inside `derive(...)`) derives the error trait.
fn derives_error(derive: &str) -> bool {
    derive
        .split(',')
        .map(str::trim)
        .any(|item| item == "Error" || item == "thiserror::Error")
}

/// The name of the `struct` or `enum` that `line` declares.
fn declared_type(line: &str) -> Option<&str> {
    let mut rest = line.trim_start();
    if let Some(after) = rest.strip_prefix("pub") {
        rest = after.trim_start();
        if rest.starts_with('(') {
            rest = rest.split_once(')')?.1.trim_start();
        }
    }
    let rest = rest
        .strip_prefix("struct ")
        .or_else(|| rest.strip_prefix("enum "))?;
    last_segment(rest.trim_start())
}

/// The error types of `text` whose name does not end in `Error`, with their 1-based line.
fn misnamed(text: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let mut pending = false;
    let mut derive = String::new();
    let mut in_derive = false;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        if in_derive {
            derive.push_str(trimmed);
            if trimmed.contains(")]") {
                in_derive = false;
                pending |= derives_error(derive.split(")]").next().unwrap_or(""));
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#[derive(") {
            if let Some((list, _)) = rest.split_once(")]") {
                pending |= derives_error(list);
            } else {
                derive.clear();
                derive.push_str(rest);
                in_derive = true;
            }
            continue;
        }
        if let Some(name) = declared_type(line) {
            if pending && !name.ends_with("Error") {
                found.push((index + 1, name.to_owned()));
            }
            pending = false;
        } else if !(trimmed.starts_with("#[") || trimmed.is_empty()) {
            pending = false;
        }
        if let Some(name) = impl_error_for(line)
            && !name.ends_with("Error")
        {
            found.push((index + 1, name.to_owned()));
        }
    }
    found
}

/// Checks every tracked `.rs` file under `crates/` and `xtask/`.
pub fn run() -> Result<bool> {
    let ws = Workspace::load()?;
    let files =
        git_lines(&ws.root, &["ls-files", "crates", "xtask"])?.context("git ls-files failed")?;
    let mut green = true;
    for file in files.iter().filter(|file| {
        file.as_str() != THIS_FILE
            && std::path::Path::new(file.as_str())
                .extension()
                .is_some_and(|ext| ext == "rs")
    }) {
        let text =
            fs::read_to_string(ws.root.join(file)).with_context(|| format!("reading {file}"))?;
        for (line, name) in misnamed(&text) {
            eprintln!(
                "error-names: {file}:{line}: `{name}` is an error type; its name ends in `Error`"
            );
            green = false;
        }
    }
    Ok(finish("check error-names", green))
}

#[cfg(test)]
mod tests {
    use super::{impl_error_for, misnamed};

    fn names(text: &str) -> Vec<String> {
        misnamed(text).into_iter().map(|(_, name)| name).collect()
    }

    #[test]
    fn an_impl_of_the_error_trait_names_its_type() {
        assert_eq!(
            impl_error_for("impl std::error::Error for ProbeFault {}"),
            Some("ProbeFault")
        );
        assert_eq!(
            impl_error_for("impl<T: Debug> Error for Wrapped<T> {"),
            Some("Wrapped")
        );
        assert_eq!(
            impl_error_for("impl error::Error for &'a Odd {}"),
            Some("Odd")
        );
        assert_eq!(impl_error_for("impl Display for ProbeFault {"), None);
        assert_eq!(impl_error_for("impl From<io::Error> for Thing {"), None);
    }

    #[test]
    fn an_error_type_not_named_for_it_is_a_hit() {
        assert_eq!(
            names("impl std::error::Error for SealFailure {}"),
            ["SealFailure"]
        );
        assert!(names("impl std::error::Error for SealError {}").is_empty());
        assert!(names("impl Error for Error {}").is_empty());
    }

    #[test]
    fn a_derive_reaches_the_type_after_its_attributes() {
        let text = "#[derive(Debug, thiserror::Error)]\n#[error(\"no\")]\n/// Docs.\npub enum PlanTrouble {\n}\n";
        assert_eq!(names(text), ["PlanTrouble"]);
        let text = "#[derive(\n    Debug,\n    Error,\n)]\npub(crate) struct Odd;\n";
        assert_eq!(names(text), ["Odd"]);
    }

    #[test]
    fn other_derives_and_other_types_are_not_hits() {
        assert!(names("#[derive(Debug, Clone)]\npub struct Plain;\n").is_empty());
        assert!(names("#[derive(Debug, thiserror::Error)]\npub struct FineError;\n").is_empty());
        assert!(names("#[derive(Debug, zbus::DBusError)]\nenum Remote {\n}\n").is_empty());
        let text = "#[derive(Debug, Error)]\nfn not_a_type() {}\npub struct Plain;\n";
        assert!(names(text).is_empty());
    }

    #[test]
    fn comments_are_not_read() {
        assert!(names("// impl Error for Whatever {}").is_empty());
    }
}
