//! The words of the sheet's serde forms that other programs read, and the password on the wire.

use porter_core::SecretText;
use porter_sheet::{FieldAnswer, FieldKind, FieldValue, RowKind, SheetInput, SignInFault};
use serde::Serialize;

fn json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("serializes")
}

#[test]
fn a_sign_in_fault_keeps_the_slug_of_an_account_already_there() {
    assert_eq!(json(&SignInFault::AlreadyAdded), r#""already_added""#);
    // Vocabulary 12: the three faults a sheet words for the sign-in of a program on this computer.
    assert_eq!(json(&SignInFault::NotRunning), r#""not_running""#);
    assert_eq!(json(&SignInFault::SignedOut), r#""signed_out""#);
    assert_eq!(json(&SignInFault::NotAllowed), r#""not_allowed""#);
}

#[test]
fn the_input_and_row_words_are_the_ones_hosts_send() {
    assert_eq!(json(&SheetInput::OpenAgain), r#"{"kind":"open_again"}"#);
    assert_eq!(json(&RowKind::Generic), r#""generic""#);
}

#[test]
fn a_typed_password_is_redacted_in_debug_and_present_on_the_wire() {
    let input = SheetInput::Submit(vec![FieldAnswer {
        kind: FieldKind::Password,
        value: FieldValue::Secret(SecretText::new("hunter2")),
    }]);
    assert!(!format!("{input:?}").contains("hunter2"));
    assert_eq!(
        json(&input),
        r#"{"kind":"submit","v":[{"kind":"password","value":{"kind":"secret","v":"hunter2"}}]}"#
    );
}
