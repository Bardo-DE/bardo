//! `cargo xtask check product-words`: the retired product names stay out of the tree.
//!
//! A word of the list is a hit when it stands alone or is one part of a snake, kebab, camel or upper-case
//! name (`Porter`, `porter_core`, `PORTER_DIR`), in the text of any tracked file (a `.rs` file, a
//! `.md`, `.toml`, `.sh`, unit file, XML, script or fixture; binary files and `Cargo.lock` are not read)
//! or in a file or directory name. `require` or `acquire` is not a hit: only whole parts count. A
//! name made of the parts `blitz` and `kit` next to each other is a hit too. The names in
//! `INSIDE_WORDS` are a hit anywhere inside a name (`libcosmic`). URLs of external
//! repositories are skipped, but a `github.com/Bardo-DE/` URL is read: a retired repository named there
//! is a hit. `THIRD-PARTY-NOTICES.md` holds other people's license texts word for word (the Mozilla
//! license names its "steward"), so its text is not read; its file name still is.

use std::fs;
use std::ops::Range;

use anyhow::{Context, Result};

use super::{finish, git_lines};
use crate::workspace::Workspace;

/// This file lists the words, so its own text is not read.
const THIS_FILE: &str = "xtask/src/tasks/product_words.rs";

/// The retired product names.
const WORDS: [&str; 25] = [
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
    "intentd",
    "maskcap",
    "memfiles",
];

/// The file whose `[[absent]]` rule names the retired crates of `INSIDE_WORDS`, so those words are
/// not read in its text (every other word is).
const ABSENT_LIST: &str = "layers.toml";

/// The file whose text is other people's license texts, copied word for word.
const LICENSE_TEXTS: &str = "THIRD-PARTY-NOTICES.md";

/// Words that are also plain English where they are not the product: a mouse wheel's `detent`.
/// They are not checked inside the text of a file (a file name is still checked).
const ENGLISH_IN_TEXT: [&str; 1] = ["detent"];

/// Retired names of the removed compositor stack and its maker (owner decision 2026-10-14). They are
/// found anywhere inside a name, in any case: `libcosmic`, `System76` and `cctk` are not split into
/// parts (`system76` holds digits), so the whole-part rule would miss them.
const INSIDE_WORDS: [&str; 3] = ["cosmic", "system76", "cctk"];

/// The retired name made of two parts: `blitz-kit`, `blitz_kit`, `BlitzKit`.
const PAIR: (&str, &str) = ("blitz", "kit");

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

/// The text of `line` without the external URLs in it. A URL of a `github.com/Bardo-DE/`
/// repository stays: the repository named there is read like any other name.
fn without_urls(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(at) = rest.find("://") {
        let scheme_start = rest[..at]
            .rfind(|c: char| !c.is_ascii_alphabetic())
            .map_or(0, |i| i + 1);
        let after = &rest[at + 3..];
        let end = after
            .find(|c: char| c.is_whitespace() || matches!(c, ')' | '>' | '"' | '\'' | '`' | ']'))
            .unwrap_or(after.len());
        if after.starts_with("github.com/Bardo-DE/") {
            out.push_str(&rest[..at + 3 + end]);
        } else {
            out.push_str(&rest[..scheme_start]);
        }
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

/// The retired words in `text`, each with the name it was found in. `in_text` also lets the
/// words that are English pass.
fn hits(text: &str, in_text: bool) -> Vec<(String, &'static str)> {
    let mut found = Vec::new();
    for range in names(text) {
        let name = &text[range.clone()];
        let parts = parts(name);
        for part in &parts {
            let Some(word) = WORDS.iter().find(|word| **word == part) else {
                continue;
            };
            if in_text && ENGLISH_IN_TEXT.contains(word) {
                continue;
            }
            found.push((name.to_owned(), *word));
        }
        let lower = name.to_ascii_lowercase();
        for word in INSIDE_WORDS {
            if lower.contains(word) {
                found.push((name.to_owned(), word));
            }
        }
        if parts
            .windows(2)
            .any(|pair| pair[0] == PAIR.0 && pair[1] == PAIR.1)
        {
            found.push((name.to_owned(), "blitz-kit"));
        }
    }
    found
}

/// Every problem in the file `file` whose text is `text`, as printable lines.
fn file_problems(file: &str, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (name, word) in hits(&file.replace(['/', '.'], " "), false) {
        out.push(format!("{file}: the file name has `{word}` (in `{name}`)"));
    }
    let license_texts = file.rsplit('/').next() == Some(LICENSE_TEXTS);
    if file == THIS_FILE || license_texts {
        return out;
    }
    for (number, line) in text.lines().enumerate() {
        let plain = without_urls(line);
        for (name, word) in hits(&plain, true) {
            if file == ABSENT_LIST && INSIDE_WORDS.contains(&word) {
                continue;
            }
            out.push(format!("{file}:{}: `{word}` (in `{name}`)", number + 1));
        }
    }
    out
}

/// Every problem in the tracked files `files` under `root`, as printable lines. A file that is not
/// UTF-8 text (an image, a font) is not read.
fn problems(root: &std::path::Path, files: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for file in files {
        if file == "Cargo.lock" || file.ends_with("/Cargo.lock") {
            continue;
        }
        let text = fs::read_to_string(root.join(file)).unwrap_or_default();
        out.extend(file_problems(file, &text));
    }
    out
}

/// Checks every tracked file: its name and, when it is text, its text.
pub fn run() -> Result<bool> {
    let ws = Workspace::load()?;
    let files = git_lines(&ws.root, &["ls-files"])?.context("git ls-files failed")?;
    let found = problems(&ws.root, &files);
    for line in &found {
        eprintln!("product-words: {line}");
    }
    Ok(finish("check product-words", found.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::{file_problems, hits, without_urls};

    fn words(text: &str, in_text: bool) -> Vec<&'static str> {
        hits(text, in_text)
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
    fn the_words_the_first_list_missed_are_hits() {
        assert_eq!(
            words("IntentdConfig and bardo-intentd", true),
            ["intentd", "intentd"]
        );
        assert_eq!(words("maskcap memfiles", true), ["maskcap", "memfiles"]);
        assert_eq!(
            words("blitz-kit blitz_kit BlitzKit", true),
            ["blitz-kit"; 3]
        );
        assert!(words("blitz-embed and a kit", true).is_empty());
    }

    #[test]
    fn the_removed_compositor_names_are_hits_inside_a_name() {
        assert_eq!(words("use libcosmic::Theme;", true), ["cosmic"]);
        assert_eq!(words("System76 and cctk", true), ["system76", "cctk"]);
        assert_eq!(words("cosmic-comp COSMIC_DIR", true), ["cosmic", "cosmic"]);
        assert!(words("a cosmos of cosmetics", true).is_empty());
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
    fn english_detent_passes_in_text_but_not_in_a_name() {
        assert!(words("one wheel detent", true).is_empty());
        assert_eq!(words("tests detent main rs", false), ["detent"]);
    }

    #[test]
    fn an_external_url_is_skipped() {
        let line = without_urls(
            "see https://example.com/stoker and https://github.com/PoHsuanLai/porter now",
        );
        assert!(words(&line, true).is_empty());
    }

    #[test]
    fn a_retired_bardo_repository_url_is_a_hit() {
        let retired = without_urls("repository = \"https://github.com/Bardo-DE/stoker\" # stoker");
        assert_eq!(words(&retired, true), ["stoker", "stoker"]);
        let current =
            without_urls("https://github.com/Bardo-DE/bardo and https://github.com/Bardo-DE/vello");
        assert!(words(&current, true).is_empty());
    }

    #[test]
    fn a_markdown_script_or_manifest_text_is_read() {
        assert_eq!(
            file_problems("crates/x/README.md", "fine\nthe porter says").len(),
            1
        );
        assert_eq!(
            file_problems("crates/x/run.sh", "exec quire-do --now").len(),
            1
        );
        assert_eq!(
            file_problems(
                "crates/x/Cargo.toml",
                "[package]\nname = \"x\"\nkeywords = [\"casement\"]"
            )
            .len(),
            1
        );
        assert!(file_problems("crates/x/README.md", "a wheel detent").is_empty());
        assert!(file_problems("layers.toml", "names = [\"cosmic-*\"]").is_empty());
        assert_eq!(
            file_problems("layers.toml", "forbid = [\"porter\"]").len(),
            1
        );
        assert_eq!(
            file_problems("crates/x/a.toml", "names = [\"cosmic-*\"]").len(),
            1
        );
    }

    #[test]
    fn a_file_name_is_read_and_license_text_is_not() {
        assert_eq!(file_problems("crates/porter/a.txt", "ok").len(), 1);
        assert!(file_problems("crates/x/THIRD-PARTY-NOTICES.md", "the license steward").is_empty());
    }
}
