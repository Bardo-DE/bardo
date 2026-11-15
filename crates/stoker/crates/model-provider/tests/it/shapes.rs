use model_provider::{ImageBytes, JsonText, ProviderError, RetrySeconds, Tokens, ToolName};

#[test]
fn image_bytes_serialise_as_base64_and_debug_hides_them() {
    let bytes = ImageBytes(vec![0xff, 0xd8, 0xff]);
    assert_eq!(serde_json::to_string(&bytes).unwrap(), r#""/9j/""#);
    assert_eq!(format!("{bytes:?}"), "ImageBytes(<3 bytes>)");
    assert!(serde_json::from_str::<ImageBytes>(r#""!!""#).is_err());
}

#[test]
fn json_text_and_tool_names_refuse_bad_input() {
    assert!(JsonText::new("{not json").is_err());
    assert!(serde_json::from_str::<JsonText>(r#""{not json""#).is_err());
    assert!(ToolName::new("").is_err());
    assert!(ToolName::new("has space").is_err());
    assert!(ToolName::new("mail.thread.archive").is_ok());
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

#[test]
fn caps_round_trip() {
    let caps = Caps {
        inputs: [InputKind::Text, InputKind::Image].into(),
        tools: ToolSupport::ServerParsed,
        output: [Constraint::JsonSchema].into(),
        reasoning: Support::Present,
        streaming: Support::Present,
        images: ImageLimits {
            per_prompt: ImageCount(3),
            rule: ResizeRule::SmartResize {
                factor: PatchFactor(32),
                min_pixels: PixelCount(65_536),
                max_pixels: PixelCount(16_777_216),
            },
            space: ModelSpace::Grid(GridMax(1000)),
        },
        context: Tokens(32768),
        max_output: Tokens(4096),
        computer_use: CuaSupport::Dialect {
            dialect: CuaDialect::Tool(ToolDialect::Holo31),
            batching: Batching::One,
            zoom: Zoom::Absent,
            profile: ActionProfile::default(),
        },
    };
    let json = serde_json::to_string(&caps).unwrap();
    assert_eq!(serde_json::from_str::<Caps>(&json).unwrap(), caps);
}
