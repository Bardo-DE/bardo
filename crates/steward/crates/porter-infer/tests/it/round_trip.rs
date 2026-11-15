//! Every wire or stored type of porter-infer survives its serde form.

use model_provider::Money;
use porter_capability::Locality;
use porter_capability::ModelId;
use porter_core::AccountId;
use porter_ids::{Permille, Tokens, TurnCount};
use porter_infer::*;
use rust_decimal::Decimal;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::Debug;

fn round_trip<T: Serialize + DeserializeOwned + PartialEq + Debug>(value: &T) {
    let json = serde_json::to_string(value).expect("serializes");
    assert_eq!(
        &serde_json::from_str::<T>(&json).expect("deserializes"),
        value,
        "{json}"
    );
}

fn served() -> ServedBy {
    ServedBy {
        account: AccountId::parse("ollama").expect("id"),
        model: ModelId::parse("llama3.2").expect("id"),
        locality: Locality::OnDevice,
    }
}

#[test]
fn a_usage_writes_a_charge_only_when_the_provider_reported_one() {
    let counted = TokenUsage {
        input: Tokens(1),
        output: Tokens(2),
        cached: Tokens(0),
        charge: None,
    };
    let json = serde_json::to_value(counted).expect("serializes");
    assert!(json.get("charge").is_none(), "{json}");
    // A payload written before the field decodes as one with no charge.
    let old = serde_json::json!({"input": 1, "output": 2, "cached": 0});
    assert_eq!(
        serde_json::from_value::<TokenUsage>(old).expect("old"),
        counted
    );
    let charged = TokenUsage {
        charge: Some(Money::usd(Decimal::new(95, 5))),
        ..counted
    };
    round_trip(&charged);
    round_trip(&ChargeTotal {
        reported: Money::usd(Decimal::new(125, 2)),
        unpriced: TurnCount(3),
    });
}

fn plain_chat_reply() -> ChatReply {
    ChatReply::new(
        "ok".into(),
        StopReason::EndTurn,
        TokenUsage {
            input: Tokens(1),
            output: Tokens(1),
            cached: Tokens(0),
            charge: None,
        },
        served(),
    )
}

#[test]
fn the_scores_of_a_reply_are_a_list_of_options_with_their_shares() {
    let mut reply = plain_chat_reply();
    reply.scores = Some(
        OptionScores::new(vec![
            OptionScore {
                option: "allow".into(),
                share: Permille(750),
            },
            OptionScore {
                option: "deny".into(),
                share: Permille(250),
            },
        ])
        .expect("a whole"),
    );
    let json = serde_json::to_value(&reply).expect("serializes");
    assert_eq!(
        json["scores"],
        serde_json::json!([
            {"option": "allow", "share": 750},
            {"option": "deny", "share": 250},
        ])
    );
}

#[test]
fn the_score_knob_is_not_written_when_off_and_reads_when_absent() {
    let control = ChatControl::new();
    let json = serde_json::to_value(&control).expect("serializes");
    assert!(json.get("scores").is_none(), "{json}");
    let back: ChatControl = serde_json::from_value(json).expect("reads");
    assert_eq!(back, control);

    let asked = control.with_scores(Knob::Set(ScoreOptions::default()));
    let json = serde_json::to_value(&asked).expect("serializes");
    assert_eq!(
        json["scores"],
        serde_json::json!({"kind": "set", "v": {"top_k": 20}})
    );
    round_trip(&asked);
}
