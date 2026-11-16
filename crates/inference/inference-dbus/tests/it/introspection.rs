//! The checked-in introspection files are the interfaces as they are declared once. A change to a
//! signature changes the file in the same commit.

use inference_dbus::inference;
use std::path::PathBuf;

#[test]
fn checked_in_introspection_matches_the_interfaces() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../dbus");
    for document in [inference::INFERENCE, inference::AGENTS] {
        let path = dir.join(document.file_name);
        let expected =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let actual = document.introspection();
        assert!(
            actual == expected,
            "{} differs from the interfaces; it should read:\n{actual}",
            path.display()
        );
    }
}

#[test]
fn speech_and_computer_use_are_fd_only() {
    let xml = inference::INFERENCE.introspection();
    for word in ["Transcribe", "Speak", "Audio", "Cua", "Listen"] {
        assert!(
            !xml.contains(word),
            "{word} must travel on the Open fd, not as a member"
        );
    }
}

#[test]
fn inference_calls_carry_an_options_dict_for_the_reserved_traceparent() {
    let xml = inference::INFERENCE.introspection();
    assert_eq!(
        xml.matches("<arg name=\"options\" type=\"a{sv}\" direction=\"in\"/>")
            .count(),
        4,
        "Availability, Open, Prepare and HardwareNotes"
    );
    assert_eq!(inference::OPTION_TRACEPARENT, "traceparent");
}

#[test]
fn computer_errors_keep_their_wire_prefix() {
    assert_eq!(
        inference::COMPUTER_ERROR_PREFIX,
        "org.bardo.Inference1.Error.Computer."
    );
}
