//! Name lookup: literal, suffix and pattern stages, weights, case, ambiguity, `__NOGLOBS__`.

use ds_helpers::mime::{MimeDb, MimeLayer, NameGuess};

use super::{mt, name};

const GLOBS: &str = "\
# fixture
50:text/plain:*.txt
50:text/x-readme:README
50:application/x-compressed-tar:*.tar.gz
50:application/gzip:*.gz
50:image/jpeg:*.jpg
50:text/x-makefile:Makefile
50:application/x-core:core.[0-9]*
50:image/x-raw:img_?.raw
50:text/x-special:special.txt
50:text/x-csrc:*.c
60:text/x-c++src:*.C:cs
50:text/x-weak:*.dup
70:text/x-strong:*.dup
50:text/x-amb-b:*.amb
50:text/x-amb-a:*.amb
50:text/x-prefixed:x.in.*t
50:text/x-neg:neg_[!a-c]
50:text/x-cs:CSONLY:cs
bad line
x:text/x-bad:*.bad
";

fn db(layers: &[&str]) -> MimeDb {
    let layers: Vec<MimeLayer<'_>> = layers
        .iter()
        .map(|g| MimeLayer {
            globs2: g,
            ..MimeLayer::default()
        })
        .collect();
    MimeDb::from_layers(&layers)
}

/// The guess a list of types stands for: none is unknown, one is that type, more are ambiguous.
fn guess(kinds: &[&str]) -> NameGuess {
    match kinds {
        [] => NameGuess::Unknown,
        [one] => NameGuess::Type(mt(one)),
        many => NameGuess::Ambiguous(many.iter().map(|kind| mt(kind)).collect()),
    }
}

#[test]
fn names_map_to_types() {
    let rows: &[(&str, &[&str])] = &[
        ("notes.txt", &["text/plain"]),
        ("NOTES.TXT", &["text/plain"]),
        ("a.tar.gz", &["application/x-compressed-tar"]),
        ("A.TAR.GZ", &["application/x-compressed-tar"]),
        ("a.gz", &["application/gzip"]),
        ("README", &["text/x-readme"]),
        ("readme", &["text/x-readme"]),
        ("Makefile", &["text/x-makefile"]),
        ("core.123", &["application/x-core"]),
        ("core.x", &[]),
        ("img_1.raw", &["image/x-raw"]),
        ("img_12.raw", &[]),
        ("special.txt", &["text/x-special"]),
        ("x.in.txt", &["text/plain"]),
        ("x.in.xt", &["text/x-prefixed"]),
        ("neg_d", &["text/x-neg"]),
        ("neg_b", &[]),
        ("foo.c", &["text/x-csrc"]),
        ("foo.C", &["text/x-c++src"]),
        ("x.dup", &["text/x-strong"]),
        // Equal patterns of equal weight are ambiguous, listed in order.
        ("x.amb", &["text/x-amb-a", "text/x-amb-b"]),
        // A case-sensitive pattern ignores other case.
        ("CSONLY", &["text/x-cs"]),
        ("csonly", &[]),
        ("a.bad", &[]),
        ("nothing", &[]),
    ];
    let db = db(&[GLOBS]);
    for (file, want) in rows {
        assert_eq!(db.by_name(&name(file)), guess(want), "{file}");
    }
}

#[test]
fn nothing_globs_clears_lower_folders_patterns_of_that_type_only() {
    let high = "0:text/plain:__NOGLOBS__\n50:text/plain:*.text\n50:image/png:*.png\n";
    let low = "50:text/plain:*.txt\n50:text/plain:*.asc\n50:image/jpeg:*.jpg\n";
    let layered = db(&[high, low]);
    let rows: &[(&str, &[&str])] = &[
        ("a.txt", &[]),
        ("a.asc", &[]),
        ("a.text", &["text/plain"]),
        ("a.png", &["image/png"]),
        ("a.jpg", &["image/jpeg"]),
    ];
    for (file, want) in rows {
        assert_eq!(layered.by_name(&name(file)), guess(want), "{file}");
    }
    // In the lower folder it leaves the higher folder's patterns alone.
    let lower = db(&["50:text/plain:*.txt\n", "0:text/plain:__NOGLOBS__\n"]);
    assert_eq!(lower.by_name(&name("a.txt")), guess(&["text/plain"]));
}
