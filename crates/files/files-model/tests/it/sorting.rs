//! Sorting and hidden-file filtering.

use std::cmp::Ordering;
use std::time::{Duration, UNIX_EPOCH};

use files_core::{
    Behaviour, ByteSize, Entry, EntryKind, FolderPrefs, MimeType, Sort, SortKey, SortOrder,
};
use files_model::{Input, arrange, compare};

use crate::common::*;

fn sort(key: SortKey, order: SortOrder) -> Sort {
    Sort { key, order }
}

fn order(entries: &[Entry], sort: Sort) -> Vec<String> {
    let prefs = FolderPrefs {
        sort,
        ..FolderPrefs::default()
    };
    arrange(entries, &prefs, true, Behaviour::default())
        .into_iter()
        .map(|at| entries[at].name.display().into_owned())
        .collect()
}

fn sized(text: &str, bytes: u64) -> Entry {
    Entry::new(name(text), EntryKind::File(ByteSize(bytes)))
}

fn mime(entry: Entry, text: &str) -> Entry {
    entry.with_mime(MimeType::new(text).unwrap())
}

#[test]
fn names_sort_naturally_and_ignoring_case() {
    let entries = [
        file("file10"),
        file("file2"),
        file("File1"),
        file("file02"),
        file("Zed"),
        file("apple"),
    ];
    let ascending = sort(SortKey::Name, SortOrder::Ascending);
    assert_eq!(
        order(&entries, ascending),
        vec!["apple", "File1", "file02", "file2", "file10", "Zed"]
    );
    let descending = sort(SortKey::Name, SortOrder::Descending);
    assert_eq!(
        order(&entries, descending),
        vec!["Zed", "file10", "file2", "file02", "File1", "apple"]
    );
}

#[test]
fn names_that_differ_only_in_case_or_zeros_have_a_fixed_order() {
    let (a, b) = (file("a"), file("A"));
    assert_eq!(compare(&a, &b, Sort::default()), Ordering::Greater);
    assert_eq!(compare(&b, &a, Sort::default()), Ordering::Less);
    assert_eq!(compare(&a, &a, Sort::default()), Ordering::Equal);
}

#[test]
fn each_key_sorts_by_its_own_rule() {
    use SortOrder::{Ascending, Descending};
    let at = |s| UNIX_EPOCH + Duration::from_secs(s);
    let kinds = vec![
        mime(file("z.txt"), "text/plain"),
        mime(file("a.png"), "image/png"),
        folder("m"),
        mime(file("b.txt"), "text/plain"),
    ];
    let sizes = vec![sized("big", 900), folder("dir"), sized("small", 5)];
    let dates = vec![
        file("new").modified_at(at(9)),
        file("unknown"),
        file("old").modified_at(at(1)),
    ];
    // (what the key is about, entries, key, order, names in order)
    let table = [
        (
            "kind: folders first, then by type",
            &kinds,
            SortKey::Kind,
            Ascending,
            vec!["m", "a.png", "b.txt", "z.txt"],
        ),
        (
            "kind reversed",
            &kinds,
            SortKey::Kind,
            Descending,
            vec!["z.txt", "b.txt", "a.png", "m"],
        ),
        (
            "size: folders count as nothing",
            &sizes,
            SortKey::Size,
            Ascending,
            vec!["dir", "small", "big"],
        ),
        (
            "modified: unknown dates first, then oldest to newest",
            &dates,
            SortKey::Modified,
            Ascending,
            vec!["unknown", "old", "new"],
        ),
    ];
    for (what, entries, key, direction, want) in table {
        assert_eq!(order(entries, sort(key, direction)), want, "{what}");
    }
    // name sort does not put folders first
    let mixed = [folder("c"), file("a"), folder("b")];
    assert_eq!(order(&mixed, Sort::default()), vec!["a", "b", "c"]);
}

#[test]
fn hidden_entries_are_left_out_until_shown() {
    let browser = loaded(vec![file(".git"), file("a"), file(".env")]);
    assert_eq!(shown(&browser), vec!["a"]);
    let (browser, _) = browser.step(Input::ToggleHidden, at(0));
    assert_eq!(shown(&browser), vec![".env", ".git", "a"]);
    let (browser, _) = browser.step(Input::ToggleHidden, at(0));
    assert_eq!(shown(&browser), vec!["a"]);
}

#[test]
fn hiding_files_drops_a_hidden_selection() {
    let browser = loaded(vec![file(".git"), file("a")]);
    let (browser, _) = run(
        browser,
        vec![
            Input::ToggleHidden,
            Input::Click(name(".git"), files_model::Modifier::None),
            Input::ToggleHidden,
        ],
    );
    assert!(selected(&browser).is_empty());
}
