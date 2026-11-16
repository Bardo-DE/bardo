use model_provider::{ImageBytes, JsonText, ProviderError, RetrySeconds, Tokens, ToolName};

#[test]
fn image_bytes_serialise_as_base64_and_debug_hides_them() {
    let bytes = ImageBytes(vec![0xff, 0xd8, 0xff]);
    assert_eq!(serde_json::to_string(&bytes).unwrap(), r#""/9j/""#);
    assert_eq!(format!("{bytes:?}"), "ImageBytes(<3 bytes>)");
    serde_json::from_str::<ImageBytes>(r#""!!""#).unwrap_err();
}

#[test]
fn json_text_and_tool_names_refuse_bad_input() {
    JsonText::new("{not json").unwrap_err();
    serde_json::from_str::<JsonText>(r#""{not json""#).unwrap_err();
    ToolName::new("").unwrap_err();
    ToolName::new("has space").unwrap_err();
    ToolName::new("mail.thread.archive").unwrap();
}

#[test]
fn provider_error_json_is_pinned() {
    let cases: &[(ProviderError, &str)] = &[
        (ProviderError::Unreachable, r#"{"kind":"unreachable"}"#),
        (
            ProviderError::RateLimited(RetrySeconds(3)),
            r#"{"kind":"rate_limited","v":3}"#,
        ),
        (
            ProviderError::ContextOverflow {
                limit: Tokens(32768),
            },
            r#"{"kind":"context_overflow","v":{"limit":32768}}"#,
        ),
    ];
    for (error, json) in cases {
        assert_eq!(&serde_json::to_string(error).unwrap(), json);
        assert_eq!(&serde_json::from_str::<ProviderError>(json).unwrap(), error);
    }
}
