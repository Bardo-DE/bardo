//! `cargo xtask check product-words`: the retired product names stay out of the tree.
//!
//! A word of the list is a hit when it stands alone or is one part of a snake, kebab, camel or upper-case
//! name (`Porter`, `porter_core`, `PORTER_DIR`), in a `.rs` file, in a file or directory name under
//! `crates/` and `xtask/`, or in a `Cargo.toml` feature or description. `require` or `acquire` is
//! not a hit: only whole parts count. URLs of external repositories are skipped.

use std::fs;
use std::ops::Range;

use anyhow::{Context, Result};

use super::{finish, git_lines};
use crate::workspace::Workspace;

/// This file lists the words, so its own text is not read.
const THIS_FILE: &str = "xtask/src/tasks/product_words.rs";

/// The retired product names.
const WORDS: [&str; 22] = [
    "quire",
    "casement",
    "detent",
    "keycap",
    "palmrest",
    "turnstile",
    "transom",
    "slipway",
    "latchkey",
    "chordkit",
    "bayonet",
    "bulkhead",
    "cua",
    "cuad",
    "porter",
    "docket",
    "almanac",
    "sill",
    "stoker",
    "steward",
    "peephole",
    "ferry",
];

/// Words that are also plain English where they are not the product: a mouse wheel's `detent`.
/// They are not checked inside `.rs` files (a name or a `Cargo.toml` line is still checked).
const ENGLISH_IN_SOURCE: [&str; 1] = ["detent"];

/// Phrases that keep a retired word on purpose, because the word is part of what is already
/// stored on a person's computer: the audit record kinds and the area name in the memory event
/// log, the actor slug, the checkpoint refs and plan hash, the keyring service and namespace names
/// the key wrap of the test keys, the memory file and export format tags, the memory key
/// schema, and the inputs of golden tests that pin them. A hit inside one of these phrases is not reported.
const KEPT_STORED_FORMS: [&str; 31] = [
    "\"cua\"",
    "cua\\\"",
    "cua.run.",
    "cua.step",
    "cua.asked",
    "cua.confirmed",
    "cua.taken_over",
    "cua.handed_back",
    "cua.procedure.",
    "cua.screen.",
    "cua.*",
    "\"docket\"",
    "\"docket.",
    "refs/docket",
    "docket@localhost",
    "docket restore point",
    "docket-restore-plan-v1",
    "\"porter\"",
    "`porter`",
    "porter-parts:",
    "porter.",
    "porter/cloud/password",
    "almanac test-keys: not a secret.",
    "\"cua.*\"",
    "quire-memory",
    "quire-memory-export-1",
    "quire-memory-export",
    "org.quire.Memory.SpaceKey",
    "org.quire.Mail",
    "org.quire.Files",
    "row(\"author\", \"Quire\")",
];

/// The parts of one name: split at `_`, `-` and digits, then at camel-case humps, lower-cased.
fn parts(name: &str) -> Vec<String> {
    let mut out = Vec::new();
    for chunk in name.split(|c: char| c == '_' || c == '-' || c.is_ascii_digit()) {
        let chars: Vec<char> = chunk.chars().collect();
        let mut start = 0;
        for i in 1..chars.len() {
            let hump = chars[i].is_uppercase()
                && (chars[i - 1].is_lowercase()
                    || (chars[i - 1].is_uppercase()
                        && chars.get(i + 1).is_some_and(|next| next.is_lowercase())));
            if hump {
                out.push(chars[start..i].iter().collect::<String>().to_lowercase());
                start = i;
            }
        }
        if start < chars.len() {
            out.push(chars[start..].iter().collect::<String>().to_lowercase());
        }
    }
    out
}

/// The text of `line` without the external URLs in it.
fn without_urls(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(at) = rest.find("://") {
        let scheme_start = rest[..at]
            .rfind(|c: char| !c.is_ascii_alphabetic())
            .map_or(0, |i| i + 1);
        out.push_str(&rest[..scheme_start]);
        let after = &rest[at + 3..];
        let end = after
            .find(|c: char| c.is_whitespace() || matches!(c, ')' | '>' | '"' | '\'' | '`' | ']'))
            .unwrap_or(after.len());
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// The byte ranges of the names (runs of letters, digits, `_` and `-`) in `text`.
fn names(text: &str) -> Vec<Range<usize>> {
    let mut found = Vec::new();
    let mut start = None;
    for (i, c) in text.char_indices() {
        let in_name = c.is_ascii_alphanumeric() || c == '_' || c == '-';
        match (in_name, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                found.push(s..i);
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        found.push(s..text.len());
    }
    found
}

/// The retired words in `text`, each with the name it was found in. `in_source` also lets the
/// words that are English pass and keeps the phrases that name stored forms.
fn hits(text: &str, in_source: bool) -> Vec<(String, &'static str)> {
    let mut found = Vec::new();
    for range in names(text) {
        let name = &text[range.clone()];
        for part in parts(name) {
            let Some(word) = WORDS.iter().find(|word| **word == part) else {
                continue;
            };
            if in_source && ENGLISH_IN_SOURCE.contains(word) {
                continue;
            }
            if in_source && kept_by_phrase(text, &range) {
                continue;
            }
            found.push((name.to_owned(), *word));
        }
    }
    found
}

/// Whether the name at `range` lies inside one of the phrases that keep a stored form.
fn kept_by_phrase(text: &str, range: &Range<usize>) -> bool {
    KEPT_STORED_FORMS.iter().any(|phrase| {
        text.match_indices(phrase)
            .any(|(at, _)| at <= range.start && range.end <= at + phrase.len())
    })
}

/// Whether the `Cargo.toml` line `line`, under table `table`, is one the check reads: a
/// description, a feature, or a `features = [...]` list.
fn is_manifest_line(table: &str, line: &str) -> bool {
    let line = line.trim_start();
    table == "features" || line.starts_with("description") || line.starts_with("features")
}

/// Every problem in the tracked files `files` under `root`, as printable lines.
fn problems(root: &std::path::Path, files: &[String]) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for file in files {
        for (name, word) in hits(&file.replace(['/', '.'], " "), false) {
            out.push(format!("{file}: the file name has `{word}` (in `{name}`)"));
        }
        let rust = std::path::Path::new(file)
            .extension()
            .is_some_and(|ext| ext == "rs");
        let manifest = std::path::Path::new(file)
            .file_name()
            .is_some_and(|n| n == "Cargo.toml");
        if (!rust && !manifest) || file == THIS_FILE {
            continue;
        }
        let text =
            fs::read_to_string(root.join(file)).with_context(|| format!("reading {file}"))?;
        let mut table = String::new();
        for (number, line) in text.lines().enumerate() {
            if manifest {
                if let Some(header) = line.trim().strip_prefix('[') {
                    table = header.trim_end_matches(']').to_owned();
                }
                if !is_manifest_line(&table, line) {
                    continue;
                }
            }
            let plain = without_urls(line);
            for (name, word) in hits(&plain, rust) {
                out.push(format!("{file}:{}: `{word}` (in `{name}`)", number + 1));
            }
        }
    }
    Ok(out)
}

/// Checks every tracked `.rs` file, `Cargo.toml` and file name under `crates/` and `xtask/`.
pub fn run() -> Result<bool> {
    let ws = Workspace::load()?;
    let files =
        git_lines(&ws.root, &["ls-files", "crates", "xtask"])?.context("git ls-files failed")?;
    let found = problems(&ws.root, &files)?;
    for line in &found {
        eprintln!("product-words: {line}");
    }
    Ok(finish("check product-words", found.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::{hits, without_urls};

    fn words(text: &str, in_source: bool) -> Vec<&'static str> {
        hits(text, in_source)
            .into_iter()
            .map(|(_, word)| word)
            .collect()
    }

    #[test]
    fn a_word_alone_or_in_a_name_is_a_hit() {
        assert_eq!(words("let porter = 1;", true), ["porter"]);
        assert_eq!(words("porter_core::Thing", true), ["porter"]);
        assert_eq!(words("porter-core", true), ["porter"]);
        assert_eq!(words("PORTER_DIR", true), ["porter"]);
    }

    #[test]
    fn a_camel_case_part_is_a_hit() {
        assert_eq!(words("struct FakeSill;", true), ["sill"]);
        assert_eq!(words("CuaAction and PorterFault", true), ["cua", "porter"]);
        assert_eq!(words("CuadConfig", true), ["cuad"]);
    }

    #[test]
    fn a_longer_word_is_not_a_hit() {
        assert!(
            words(
                "require acquire Requirement viewporter exporter silly",
                true
            )
            .is_empty()
        );
        assert!(words("TransportError and ExportError", true).is_empty());
    }

    #[test]
    fn a_kept_stored_form_is_not_a_hit() {
        assert!(words("kind(\"cua.run.started\")", true).is_empty());
        assert!(words("const REF: &str = \"refs/docket/checkpoints\";", true).is_empty());
        assert!(words("Namespace::parse(\"porter.accountd\")", true).is_empty());
        assert_eq!(
            words("let docket = 1; // docket.call", true),
            ["docket", "docket"]
        );
    }

    #[test]
    fn english_detent_passes_in_source_but_not_in_a_name() {
        assert!(words("one wheel detent", true).is_empty());
        assert_eq!(words("tests detent main rs", false), ["detent"]);
    }

    #[test]
    fn an_external_url_is_skipped() {
        let line = without_urls("repository = \"https://github.com/Bardo-DE/stoker\" # stoker");
        assert_eq!(words(&line, true), ["stoker"]);
    }
}
