//! The types the rig amendment added to the frozen interface: round trips, pinned JSON and the
//! tables that are pure data.

use model_provider::{
    Dims, EmbedEnd, EmbedRole, EmbedTurn, EmbedVector, FieldName, Knob, ModelName, ProviderError,
    ServerStatus, StopReason, TurnUsage, VariantName,
};

fn round_trip<T>(value: &T, json: &str)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + core::fmt::Debug,
{
    assert_eq!(serde_json::to_string(value).unwrap(), json);
    assert_eq!(&serde_json::from_str::<T>(json).unwrap(), value);
}

#[test]
fn a_server_error_carries_only_its_status() {
    round_trip(
        &ProviderError::Server(ServerStatus(503)),
        r#"{"kind":"server","v":503}"#,
    );
}

#[test]
fn stop_reasons_map_to_the_convention_finish_reasons() {
    use model_genai_names::Finish;
    const CASES: &[(StopReason, Finish)] = &[
        (StopReason::EndTurn, Finish::Stop),
        (StopReason::StopSequence, Finish::Stop),
        (StopReason::ToolUse, Finish::ToolCalls),
        (StopReason::MaxTokens, Finish::Length),
        (StopReason::ContentFilter, Finish::ContentFilter),
        (StopReason::Unrecognized, Finish::Error),
    ];
    for (stop, finish) in CASES {
        assert_eq!(Finish::from(*stop), *finish);
    }
}

#[test]
fn embedding_types_round_trip_and_the_inputs_do_not_print() {
    let turn = EmbedTurn {
        model: ModelName("nomic-embed-text".into()),
        inputs: vec!["a private note".into()],
        role: EmbedRole::Document,
        dims: Knob::Set(Dims(768)),
    };
    round_trip(
        &turn,
        r#"{"model":"nomic-embed-text","inputs":["a private note"],"role":"document","dims":{"kind":"set","v":768}}"#,
    );
    assert!(!format!("{turn:?}").contains("private"));
    let end = EmbedEnd {
        vectors: vec![EmbedVector(vec![0.5, -0.25])],
        usage: TurnUsage::default(),
        served: ModelName("nomic-embed-text".into()),
    };
    let json = serde_json::to_string(&end).unwrap();
    assert_eq!(serde_json::from_str::<EmbedEnd>(&json).unwrap(), end);
    round_trip(&EmbedRole::Query, r#""query""#);
}

#[test]
fn field_and_variant_names_are_checked_where_they_enter() {
    assert!(FieldName::new("verdict_2").is_ok());
    assert!(FieldName::new("_x").is_ok());
    assert!(FieldName::new("").is_err());
    assert!(FieldName::new("2nd").is_err());
    assert!(FieldName::new("has space").is_err());
    assert!(FieldName::new("quote\"").is_err());
    assert!(FieldName::new("x".repeat(65)).is_err());
    assert!(serde_json::from_str::<VariantName>(r#""a-b""#).is_err());
}
