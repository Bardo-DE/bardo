use genai_names::attr;

#[test]
fn attribute_keys_are_unique_and_namespaced() {
    let mut seen = std::collections::BTreeSet::new();
    for key in attr::ALL {
        assert!(seen.insert(*key), "{key} is listed twice");
        assert!(
            key.starts_with("gen_ai.") || matches!(*key, "error.type" | "server.address"),
            "{key} is outside the conventions' namespaces"
        );
    }
    assert_eq!(attr::ALL.len(), 19);
}

#[test]
fn no_content_attribute_exists() {
    // Prompts, completions and tool arguments are personal: they have no key here.
    for key in attr::ALL {
        for forbidden in [
            "input.messages",
            "output.messages",
            "system_instructions",
            "call.arguments",
            "call.result",
        ] {
            assert!(!key.contains(forbidden), "{key} would carry content");
        }
    }
}
