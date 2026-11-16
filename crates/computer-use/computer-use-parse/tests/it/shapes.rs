use computer_use_parse::VerbText;

#[test]
fn verb_text_refuses_long_and_control_input() {
    VerbText::new("a".repeat(65)).unwrap_err();
    VerbText::new("a\nb").unwrap_err();
    VerbText::new("a".repeat(64)).unwrap();
}
