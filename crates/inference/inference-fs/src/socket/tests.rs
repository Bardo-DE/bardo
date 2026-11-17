use super::*;
use base_testkit::Scratch;
use std::os::unix::net::UnixListener;

#[test]
fn a_socket_path_of_108_bytes_is_refused_and_107_is_not() {
    let scratch = Scratch::new("startup-len");
    let base = scratch.path().to_string_lossy().len();
    // The path is the scratch directory, a slash and a name: 107 and 108 bytes in all.
    let name = |total: usize| "n".repeat(total - base - 1);
    let fits = scratch.path().join(name(107));
    let too_long = scratch.path().join(name(108));
    assert_eq!(fits.as_os_str().len(), 107);
    assert_eq!(clear_socket(&fits), Ok(()));
    assert_eq!(
        clear_socket(&too_long),
        Err(Cause::SocketPathTooLong {
            path: too_long.clone(),
            len: 108
        })
    );
    // The bytes count, not the characters.
    let wide = scratch.path().join("é".repeat((108 - base) / 2));
    assert!(wide.as_os_str().len() >= SUN_PATH);
    assert!(matches!(
        clear_socket(&wide),
        Err(Cause::SocketPathTooLong { .. })
    ));
}

#[test]
fn a_stale_socket_is_removed_and_anything_else_is_kept() {
    let scratch = Scratch::new("startup-clear");
    let dir = scratch.path();

    let absent = dir.join("absent.sock");
    assert_eq!(clear_socket(&absent), Ok(()));

    // A socket an earlier run left behind: bound, then the owner died (the file stays).
    let stale = dir.join("stale.sock");
    drop(UnixListener::bind(&stale).expect("bind"));
    assert!(stale.exists());
    assert_eq!(clear_socket(&stale), Ok(()));
    assert!(!stale.exists());

    // A regular file is refused and kept, with its contents.
    let file = dir.join("file.sock");
    std::fs::write(&file, b"mine").expect("write");
    assert_eq!(
        clear_socket(&file),
        Err(Cause::SocketPathTaken { path: file.clone() })
    );
    assert_eq!(std::fs::read(&file).expect("kept"), b"mine");

    // So is a directory, and a symlink (even one that points at a socket).
    let folder = dir.join("folder.sock");
    std::fs::create_dir(&folder).expect("dir");
    assert!(matches!(
        clear_socket(&folder),
        Err(Cause::SocketPathTaken { .. })
    ));
    let target = dir.join("target.sock");
    let _live = UnixListener::bind(&target).expect("bind");
    let link = dir.join("link.sock");
    std::os::unix::fs::symlink(&target, &link).expect("symlink");
    assert!(matches!(
        clear_socket(&link),
        Err(Cause::SocketPathTaken { .. })
    ));
    assert!(target.exists(), "the socket behind a link is not touched");
}
