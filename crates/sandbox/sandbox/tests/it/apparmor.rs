//! Why a refused user namespace is refused: the `AppArmor` switch, read from a scratch file.

use sandbox::{CannotSandboxError, namespaces_refused, refusal_at};

#[test]
fn the_switch_decides_the_reason() {
    let table = [
        (Some("1\n"), CannotSandboxError::RestrictedByAppArmor),
        (Some("1"), CannotSandboxError::RestrictedByAppArmor),
        (Some("0\n"), CannotSandboxError::NamespacesDenied),
        (Some(""), CannotSandboxError::NamespacesDenied),
        (Some("garbage"), CannotSandboxError::NamespacesDenied),
        (None, CannotSandboxError::NamespacesDenied),
    ];
    for (text, want) in table {
        assert_eq!(namespaces_refused(text), want, "switch {text:?}");
    }
}

#[test]
fn the_switch_file_is_read_from_the_path_given() {
    let scratch = tempfile::tempdir().expect("scratch");
    let file = scratch.path().join("restrict");
    assert_eq!(refusal_at(&file), CannotSandboxError::NamespacesDenied);
    std::fs::write(&file, "1\n").expect("write");
    assert_eq!(refusal_at(&file), CannotSandboxError::RestrictedByAppArmor);
    std::fs::write(&file, "0\n").expect("write");
    assert_eq!(refusal_at(&file), CannotSandboxError::NamespacesDenied);
}
