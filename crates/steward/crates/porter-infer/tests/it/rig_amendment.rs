//! The wire types the rig amendment added: pinned JSON, round trips, and the redaction rules.

use porter_ids::{Count, Permille, Tokens};
use porter_infer::*;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::Debug;

fn pinned<T: Serialize + DeserializeOwned + PartialEq + Debug>(value: &T, json: &str) {
    assert_eq!(serde_json::to_string(value).expect("serializes"), json);
    assert_eq!(
        &serde_json::from_str::<T>(json).expect("deserializes"),
        value
    );
}

#[test]
fn chat_control_has_pinned_json() {
    let control = ChatControl::new()
        .with_tool_choice(ToolChoice::Named(
            ToolName::new("mail.search").expect("name"),
        ))
        .with_max_output(Knob::Set(Tokens(1)))
        .with_reasoning(Reasoning::On(Effort::Low))
        .with_sampling(Knob::Set(Sampling {
            temperature: Permille(700),
            top_p: Knob::Off,
            top_k: Knob::Set(Count(20)),
            min_p: Knob::Off,
            seed: Knob::Set(Seed(7)),
        }))
        .with_stop(vec!["END".into()]);
    pinned(
        &control,
        r#"{"tool_choice":{"kind":"named","v":"mail.search"},"tool_calls":"one","max_output":{"kind":"set","v":1},"reasoning":{"kind":"on","v":"low"},"sampling":{"kind":"set","v":{"temperature":700,"top_p":{"kind":"off"},"top_k":{"kind":"set","v":20},"min_p":{"kind":"off"},"seed":{"kind":"set","v":7}}},"stop":["END"]}"#,
    );
}

#[test]
fn a_thought_part_carries_its_seal_and_redacts() {
    let part = MessagePart::Thought(ThoughtPart {
        text: "hm".into(),
        seal: ThoughtSeal::Signed(SignatureText("sig".into())),
    });
    pinned(
        &part,
        r#"{"kind":"thought","v":{"text":"hm","seal":{"kind":"signed","v":"sig"}}}"#,
    );
    pinned(&ThoughtSeal::None, r#"{"kind":"none"}"#);
    pinned(
        &ThoughtSeal::Redacted(OpaqueText("enc".into())),
        r#"{"kind":"redacted","v":"enc"}"#,
    );
    assert_eq!(
        format!("{:?}", SignatureText("secret".into())),
        "SignatureText(<6 bytes>)"
    );
}

#[test]
fn open_options_pin_their_json_and_refuse_what_is_not_one() {
    use porter_core::consent::Usage;
    use porter_infer::{Asker, ModelRef};
    let parent =
        Traceparent::parse("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01").expect("ok");
    let helper = porter_core::HelperId::parse("h-1").expect("helper");
    let model = ModelRef::parse("this-computer/qwen3-4b").expect("model");
    let rows = [
        (
            OpenOptions::default().with_traceparent(parent),
            r#"{"traceparent":"00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"}"#,
        ),
        (OpenOptions::default(), r#"{"traceparent":null}"#),
        (
            OpenOptions::default()
                .with_asker(Asker::Helper(helper.clone()))
                .with_model(model.clone()),
            r#"{"traceparent":null,"asker":{"helper":"h-1"},"model":{"account":"this-computer","model":"qwen3-4b"}}"#,
        ),
        (
            OpenOptions::default().with_asker(Asker::Person),
            r#"{"traceparent":null,"asker":"person"}"#,
        ),
        (
            OpenOptions::default().with_usage(Usage::Background),
            r#"{"traceparent":null,"usage":"background"}"#,
        ),
        (
            OpenOptions::default().with_usage(Usage::Interactive),
            r#"{"traceparent":null,"usage":"interactive"}"#,
        ),
    ];
    for (options, json) in rows {
        pinned(&options, json);
    }
    // Nothing set reads as the person, interactive.
    let plain = OpenOptions::default();
    assert_eq!(
        (
            plain.asker_or_default(),
            plain.usage_or_default(),
            plain.model.clone()
        ),
        (Asker::Person, Usage::Interactive, None)
    );
    for refused in [r#"{"traceparent":"nope"}"#, r#"{"usage":"batch"}"#] {
        assert!(
            serde_json::from_str::<OpenOptions>(refused).is_err(),
            "{refused}"
        );
    }
    // The bus's text form round-trips and refuses what is not one.
    for asker in [Asker::Person, Asker::Helper(helper)] {
        assert_eq!(Asker::from_text(&asker.to_text()), Some(asker));
    }
    assert_eq!(Asker::from_text("helper:"), None);
    assert_eq!(Asker::from_text("someone"), None);
    assert_eq!(ModelRef::parse(&model.to_string()), Some(model));
    assert_eq!(ModelRef::parse("no-slash"), None);
}
