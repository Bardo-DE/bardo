use computer_use_parse::VerbText;

#[test]
fn verb_text_refuses_long_and_control_input() {
    assert!(VerbText::new("a".repeat(65)).is_err());
    assert!(VerbText::new("a\nb").is_err());
    assert!(VerbText::new("a".repeat(64)).is_ok());
}
