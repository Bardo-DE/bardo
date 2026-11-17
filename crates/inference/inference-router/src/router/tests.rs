use super::*;
use account_capability::Billing;
use account_capability::ModelId;
use account_capability::{ComputerUseBatching, ComputerUseCap, ComputerUseEnv, LlmCap, LlmWire};
use account_core::AccountId;
use account_match::{ComputerUseNeed, LlmNeed};
use base_ids::{Px, Tokens};
use inference_core::{AutoMode, AutoRow, TierRow};

fn llm_cap() -> Capability {
    Capability::Llm(LlmCap {
        features: [LlmFeature::Chat, LlmFeature::Tools].into(),
        context: Tokens(8192),
        max_output: Tokens(1024),
        wire: LlmWire::ChatCompletions,
    })
}

fn card(account: &str, model: &str, locality: Locality, capability: Capability) -> ModelCard {
    ModelCard {
        account: AccountId::parse(account).expect("id"),
        model: ModelId::parse(model).expect("id"),
        locality,
        billing: Billing::Free,
        capabilities: vec![capability],
    }
}

fn listed(card: ModelCard, readiness: Readiness) -> Listed {
    Listed::new(card, readiness, SwapCost::Resident, LicenceClass::Open)
}

fn local(model: &str) -> ModelCard {
    card("local", model, Locality::OnDevice, llm_cap())
}

fn cloud(model: &str) -> ModelCard {
    card(
        "anthropic",
        model,
        Locality::Cloud { region: None },
        llm_cap(),
    )
}

fn need() -> Need {
    Need::Llm(LlmNeed::new([LlmFeature::Chat].into(), Tokens(1000)))
}

fn computer_use_need() -> Need {
    Need::ComputerUse(ComputerUseNeed::new([ComputerUseEnv::Desktop].into()))
}

fn computer_use_cap() -> Capability {
    Capability::ComputerUse(ComputerUseCap {
        environments: [ComputerUseEnv::Desktop].into(),
        batching: ComputerUseBatching::One,
        zoom: false,
        max_image: Px(1024),
        wire: LlmWire::ChatCompletions,
    })
}

fn identity() -> Need {
    Need::Identity(account_match::IdentityNeed::new())
}

fn off() -> Policy {
    Policy {
        local_only: false,
        ..Policy::proposed()
    }
}

fn pick(
    need: &Need,
    class: DataClass,
    models: &[Listed],
    policy: &Policy,
) -> Result<(String, Readiness), InferRefusedError> {
    choose(
        need,
        class,
        Tier::Balanced,
        models,
        policy,
        &TierMap::default(),
        AutoPolicy::default(),
    )
    .map(|d| (d.chosen.model.as_str().to_owned(), d.readiness))
    .map_err(|r| r.refusal)
}

#[expect(
    clippy::too_many_lines,
    reason = "one table of routes; each row is a short need and its expected decision"
)]
#[test]
fn the_route_table() {
    let ready = Readiness::Ready;
    #[allow(
        clippy::type_complexity,
        reason = "one table of (name, need, the route inputs, the expected decision)"
    )]
    let cases: Vec<(
        &str,
        Need,
        DataClass,
        Vec<Listed>,
        Policy,
        Result<(&str, Readiness), InferRefusedError>,
    )> = vec![
        (
            "a local model that fits",
            need(),
            DataClass::Notes,
            vec![listed(local("a"), ready)],
            Policy::proposed(),
            Ok(("a", ready)),
        ),
        (
            "readiness rides along",
            need(),
            DataClass::Notes,
            vec![listed(local("a"), Readiness::Loadable)],
            Policy::proposed(),
            Ok(("a", Readiness::Loadable)),
        ),
        (
            "nothing installed",
            need(),
            DataClass::Notes,
            vec![],
            Policy::proposed(),
            Err(InferRefusedError::Unavailable),
        ),
        (
            "weights not downloaded cannot answer",
            need(),
            DataClass::Notes,
            vec![listed(local("a"), Readiness::Downloadable)],
            Policy::proposed(),
            Err(InferRefusedError::Unavailable),
        ),
        (
            "a failed engine cannot answer",
            need(),
            DataClass::Notes,
            vec![listed(local("a"), Readiness::Unavailable)],
            Policy::proposed(),
            Err(InferRefusedError::Unavailable),
        ),
        (
            "local-only removes the cloud",
            need(),
            DataClass::Notes,
            vec![listed(cloud("c"), ready)],
            Policy::proposed(),
            Err(InferRefusedError::Unavailable),
        ),
        (
            "the prompt class is refused a cloud-only route by its floor",
            need(),
            DataClass::Prompt,
            vec![listed(cloud("c"), ready)],
            off(),
            Err(InferRefusedError::RequiresCloud(DataClass::Prompt)),
        ),
        (
            "so is every personal class",
            need(),
            DataClass::Mail,
            vec![listed(cloud("c"), ready)],
            off(),
            Err(InferRefusedError::RequiresCloud(DataClass::Mail)),
        ),
        (
            "public data may go to a cloud that nobody has granted yet",
            need(),
            DataClass::Public,
            vec![listed(cloud("c"), ready)],
            off(),
            Err(InferRefusedError::NeedsGrant),
        ),
        (
            "this computer is preferred to the cloud",
            need(),
            DataClass::Public,
            vec![listed(cloud("c"), ready), listed(local("a"), ready)],
            off(),
            Ok(("a", ready)),
        ),
        (
            "a prompt goes to the local model beside a cloud one",
            need(),
            DataClass::Prompt,
            vec![listed(cloud("c"), ready), listed(local("a"), ready)],
            off(),
            Ok(("a", ready)),
        ),
        (
            "a need no model meets",
            Need::Llm(LlmNeed::new([LlmFeature::Vision].into(), Tokens(1000))),
            DataClass::Notes,
            vec![listed(local("a"), ready)],
            Policy::proposed(),
            Err(InferRefusedError::Unavailable),
        ),
        (
            "computer use takes screen data only",
            computer_use_need(),
            DataClass::Mail,
            vec![listed(
                card("local", "u", Locality::OnDevice, computer_use_cap()),
                ready,
            )],
            Policy::proposed(),
            Err(InferRefusedError::Unsupported),
        ),
        (
            "computer use on a screen",
            computer_use_need(),
            DataClass::Screen,
            vec![listed(
                card("local", "u", Locality::OnDevice, computer_use_cap()),
                ready,
            )],
            Policy::proposed(),
            Ok(("u", ready)),
        ),
        (
            "a need that is not AI",
            identity(),
            DataClass::Notes,
            vec![listed(local("a"), ready)],
            Policy::proposed(),
            Err(InferRefusedError::Unsupported),
        ),
    ];
    for (name, need, class, models, policy, expected) in cases {
        let expected = expected.map(|(model, readiness)| (model.to_owned(), readiness));
        assert_eq!(pick(&need, class, &models, &policy), expected, "{name}");
    }
}

#[test]
fn the_users_tier_choice_decides_between_models_that_fit() {
    let models = [
        listed(local("alpha"), Readiness::Ready),
        listed(local("beta"), Readiness::Ready),
    ];
    let tiers = |model: &str| TierMap {
        rows: vec![TierRow {
            kind: Slot::Text,
            tier: Tier::Best,
            model: ModelRef {
                account: AccountId::parse("local").expect("id"),
                model: ModelId::parse(model).expect("id"),
            },
        }],
        autos: vec![],
        pools: vec![],
    };
    let route = |tier: Tier, map: &TierMap| {
        choose(
            &need(),
            DataClass::Notes,
            tier,
            &models,
            &Policy::proposed(),
            map,
            AutoPolicy::default(),
        )
        .map(|d| d.chosen.model.as_str().to_owned())
        .map_err(|r| r.refusal)
    };
    assert_eq!(route(Tier::Best, &tiers("beta")).as_deref(), Ok("beta"));
    assert_eq!(route(Tier::Best, &tiers("alpha")).as_deref(), Ok("alpha"));
    // A tier with no row of its own is served by the person's pick for another tier of the slot,
    // not by the first listed.
    assert_eq!(route(Tier::Fast, &tiers("beta")).as_deref(), Ok("beta"));
    assert_eq!(route(Tier::Balanced, &tiers("beta")).as_deref(), Ok("beta"));
}

#[test]
fn needs_map_to_the_picker_kinds() {
    use account_capability::SpeechMode;
    use account_match::{DimsNeed, EmbedNeed, SpeechNeed};
    let speech =
        |modes: &[SpeechMode]| Need::Speech(SpeechNeed::new(modes.iter().copied().collect()));
    let cases = [
        (need(), Some(Slot::Text)),
        (computer_use_need(), Some(Slot::ComputerUse)),
        (
            Need::Embeddings(EmbedNeed::new(
                DimsNeed::Any,
                std::collections::BTreeSet::new(),
            )),
            Some(Slot::Embeddings),
        ),
        (speech(&[SpeechMode::Stt]), Some(Slot::VoiceIn)),
        (speech(&[SpeechMode::Tts]), Some(Slot::VoiceOut)),
        (
            speech(&[SpeechMode::Stt, SpeechMode::Tts]),
            Some(Slot::VoiceOut),
        ),
        (identity(), None),
    ];
    for (need, kind) in cases {
        assert_eq!(slot_of_need(&need), kind);
    }
}

fn named(model: &str) -> ModelRef {
    ModelRef {
        account: AccountId::parse("local").expect("id"),
        model: ModelId::parse(model).expect("id"),
    }
}

fn decide(models: &[Listed], tiers: &TierMap, auto: AutoPolicy) -> Result<Decided, PickRefusal> {
    choose(
        &need(),
        DataClass::Notes,
        Tier::Balanced,
        models,
        &Policy::proposed(),
        tiers,
        auto,
    )
}

fn auto_map() -> TierMap {
    TierMap {
        rows: vec![],
        autos: vec![AutoRow {
            kind: Slot::Text,
            tier: Tier::Balanced,
            mode: AutoMode::WarmFirst,
        }],
        pools: vec![inference_core::PoolRow {
            kind: Slot::Text,
            chooser: inference_core::Chooser::Assistant,
            models: ["a", "alpha", "beta"]
                .map(named)
                .into_iter()
                .chain([ModelRef {
                    account: AccountId::parse("anthropic").expect("id"),
                    model: ModelId::parse("claude").expect("id"),
                }])
                .collect(),
        }],
    }
}

#[test]
fn an_empty_row_says_why_in_catalogue_terms() {
    let one = [listed(local("alpha"), Readiness::Loadable)];
    let two = [
        listed(local("alpha"), Readiness::Loadable),
        listed(local("beta"), Readiness::Ready),
    ];
    let none = TierMap::default();
    let only = decide(&one, &none, AutoPolicy::default()).expect("one");
    assert_eq!(only.why, Why::OnlyOne);
    let first = decide(&two, &none, AutoPolicy::default()).expect("first");
    assert_eq!(
        (first.chosen.model.as_str(), first.why),
        ("alpha", Why::CatalogueOrder)
    );
}

#[test]
fn automatic_prefers_the_loaded_model_and_says_so() {
    let models = [
        listed(local("alpha"), Readiness::Loadable),
        listed(local("beta"), Readiness::Ready),
    ];
    let got = decide(&models, &auto_map(), AutoPolicy::default()).expect("picks");
    assert_eq!((got.chosen.model.as_str(), got.why), ("beta", Why::Warm));
    assert_eq!(got.readiness, Readiness::Ready);
}

#[test]
fn automatic_names_the_idle_model_it_unloads_and_only_when_allowed() {
    let swapping = Listed {
        swap: SwapCost::Evicts {
            victim: named("old"),
            load: inference_core::EngineLoad::Idle,
            cold_start_estimate_s: 90,
        },
        ..listed(local("alpha"), Readiness::Loadable)
    };
    let got = decide(
        std::slice::from_ref(&swapping),
        &auto_map(),
        AutoPolicy::default(),
    )
    .expect("swaps");
    assert_eq!(
        got.why,
        Why::Evicted {
            model: named("old")
        }
    );
    let never = AutoPolicy {
        allow_evict: inference_core::AutoEvict::Never,
        ..AutoPolicy::default()
    };
    assert_eq!(
        decide(&[swapping], &auto_map(), never)
            .expect_err("no swap")
            .refusal,
        InferRefusedError::Unavailable
    );
}

#[test]
fn automatic_takes_the_cloud_only_when_the_floor_and_local_only_allow() {
    let models = [listed(cloud("claude"), Readiness::Ready)];
    // Notes data stays on this computer by default: not even Automatic widens the floor.
    assert_eq!(
        decide(&models, &auto_map(), AutoPolicy::default())
            .expect_err("floor")
            .refusal,
        InferRefusedError::Unavailable
    );
    let open = choose(
        &need(),
        DataClass::Public,
        Tier::Balanced,
        &models,
        &off(),
        &auto_map(),
        AutoPolicy::default(),
    )
    .expect_err("a cloud model needs a grant, which only accountd can give and cannot yet");
    // `admit` lets Public data reach the cloud; consent is the rule that stops it today.
    assert_eq!(open.refusal, InferRefusedError::NeedsGrant);
}

#[test]
fn a_named_model_that_cannot_serve_refuses_naming_it_and_no_other_answers() {
    let models = [
        listed(local("alpha"), Readiness::Ready),
        listed(local("beta"), Readiness::Downloadable),
    ];
    let tiers = TierMap {
        rows: vec![TierRow {
            kind: Slot::Text,
            tier: Tier::Balanced,
            model: named("beta"),
        }],
        autos: vec![],
        pools: vec![],
    };
    let err = decide(&models, &tiers, AutoPolicy::default()).expect_err("refuses");
    assert_eq!(err.refusal, InferRefusedError::Unavailable);
    assert_eq!(
        err.declined,
        Some(Declined {
            model: named("beta"),
            because: DeclinedBecause::NotInstalled
        })
    );
}

#[test]
fn a_text_only_model_named_for_computer_use_is_refused_by_name() {
    let models = [
        listed(local("blind"), Readiness::Ready),
        listed(
            card("local", "sighted", Locality::OnDevice, computer_use_cap()),
            Readiness::Ready,
        ),
    ];
    let tiers = TierMap::default().naming(Slot::ComputerUse, &named("blind"));
    let ask = |tiers: &TierMap| {
        choose(
            &computer_use_need(),
            DataClass::Screen,
            Tier::Balanced,
            &models,
            &Policy::proposed(),
            tiers,
            AutoPolicy::default(),
        )
    };
    let err = ask(&tiers).expect_err("a model that cannot see the screen");
    assert_eq!(
        err.declined,
        Some(Declined {
            model: named("blind"),
            because: DeclinedBecause::CannotSeeScreen,
        })
    );
    let served = ask(&TierMap::default().naming(Slot::ComputerUse, &named("sighted")))
        .expect("a model that reads images is served");
    assert_eq!(served.chosen.model.as_str(), "sighted");
}

#[test]
fn computer_use_never_picks_a_model_that_cannot_read_images_even_if_its_card_claims_it() {
    let claims_without_sight = ModelCard {
        capabilities: vec![llm_cap(), computer_use_cap()],
        ..local("claims")
    };
    let sees = ModelCard {
        capabilities: vec![
            Capability::Llm(LlmCap {
                features: [LlmFeature::Chat, LlmFeature::Vision].into(),
                context: Tokens(8192),
                max_output: Tokens(1024),
                wire: LlmWire::ChatCompletions,
            }),
            computer_use_cap(),
        ],
        ..local("sees")
    };
    let ask = |models: &[Listed]| {
        choose(
            &computer_use_need(),
            DataClass::Screen,
            Tier::Balanced,
            models,
            &Policy::proposed(),
            &TierMap::default(),
            AutoPolicy::default(),
        )
    };
    let both = [
        listed(claims_without_sight.clone(), Readiness::Ready),
        listed(sees, Readiness::Ready),
    ];
    assert_eq!(
        ask(&both).expect("the one that sees").chosen.model.as_str(),
        "sees"
    );
    let only_claim = [listed(claims_without_sight, Readiness::Ready)];
    assert_eq!(
        ask(&only_claim).expect_err("none sees").refusal,
        InferRefusedError::NoModelSeesScreen
    );
}
