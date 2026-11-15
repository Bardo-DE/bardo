//! `AbsPath`: absolute, normalised, never stepping up.

use bulkhead::{AbsPath, PathFault, RootState};

#[test]
fn dots_and_doubled_slashes_fold_away_and_only_slash_is_the_root() {
    let path = AbsPath::parse("/a//b/./c/").expect("path");
    assert_eq!(path.as_str(), "/a/b/c");
    assert_eq!(
        AbsPath::parse("/").expect("root").is_root(),
        RootState::Root
    );
    assert_eq!(AbsPath::parse("/a").expect("a").is_root(), RootState::Below);
}

#[test]
fn relative_parent_and_control_are_refused() {
    assert_eq!(AbsPath::parse("a/b"), Err(PathFault::NotAbsolute));
    assert_eq!(AbsPath::parse("/a/../b"), Err(PathFault::ParentStep));
    assert_eq!(AbsPath::parse("/a\nb"), Err(PathFault::Control));
}
