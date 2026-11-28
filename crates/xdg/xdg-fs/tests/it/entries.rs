//! Desktop entries on scratch folders: the walk and its ids, and a window finding its entry on
//! disk by the same rules.

use std::fs;
use std::path::PathBuf;
use xdg_core::entry_for_window;
use xdg_fs::{first_of_each_id, load_entries, walk_applications};

/// One test's own directory, removed on drop.
struct Scratch(base_testkit::Scratch);

impl Scratch {
    fn new(tag: &str) -> Self {
        Scratch(base_testkit::Scratch::new(tag))
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().expect("parent")).expect("folders");
        fs::write(&path, text).expect("write");
        path
    }
}

#[test]
fn the_walk_recurses_in_order_ends_at_a_loop_and_the_first_folder_shadows() {
    let first = Scratch::new("first");
    let second = Scratch::new("second");
    first.write("b.desktop", "");
    first.write("a.desktop", "");
    first.write("notes.txt", "");
    first.write("kde/c.desktop", "");
    second.write("a.desktop", "");
    second.write("z.desktop", "");
    std::os::unix::fs::symlink(&first.0, first.0.join("kde/loop")).expect("link");
    let roots = [
        first.0.to_path_buf(),
        second.0.join("missing"),
        second.0.to_path_buf(),
    ];

    let all = walk_applications(&roots);
    let ids: Vec<&str> = all.iter().map(|file| file.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "a.desktop",
            "b.desktop",
            "kde-c.desktop",
            "a.desktop",
            "z.desktop"
        ]
    );
    let once = first_of_each_id(all);
    let got: Vec<(&str, bool)> = once
        .iter()
        .map(|file| (file.id.as_str(), file.path.starts_with(&first.0)))
        .collect();
    let want = [
        ("a.desktop", true),
        ("b.desktop", true),
        ("kde-c.desktop", true),
        ("z.desktop", false),
    ];
    assert_eq!(got, want);
}

#[test]
fn a_window_finds_its_entry_on_disk_by_the_same_rules() {
    let first = Scratch::new("window-first");
    let second = Scratch::new("window-second");
    first.write("org.a.Exact.desktop", "[Desktop Entry]\nIcon=exact\n");
    first.write("kde/tool.desktop", "[Desktop Entry]\nIcon=in-folder\n");
    first.write(
        "zzz.desktop",
        "[Desktop Entry]\nIcon=by-class\nStartupWMClass=Classy\n",
    );
    first.write("dup.desktop", "[Desktop Entry]\nIcon=first\n");
    first.write("broken.desktop", "no group here\n");
    second.write("dup.desktop", "[Desktop Entry]\nIcon=second\n");
    second.write(
        "org.b.Shop.desktop",
        "[Desktop Entry]\nIcon=shop\nX-Flatpak=com.shop.App\n",
    );
    let entries = load_entries(&[first.0.to_path_buf(), second.0.to_path_buf()]);
    // name, app id, icon
    let cases = [
        ("the file named so", "org.a.Exact", Some("exact")),
        ("a folder's dashed id", "kde-tool", Some("in-folder")),
        ("the class", "Classy", Some("by-class")),
        ("the first folder wins", "dup", Some("first")),
        ("the flatpak id", "com.shop.App", Some("shop")),
        ("a file with no entry group", "broken", None),
        ("none", "nothing", None),
    ];
    for (name, app_id, icon) in cases {
        let got =
            entry_for_window(&entries, app_id, None).and_then(xdg_core::ApplicationEntry::icon);
        assert_eq!(got.as_deref(), icon, "{name}");
    }
}
