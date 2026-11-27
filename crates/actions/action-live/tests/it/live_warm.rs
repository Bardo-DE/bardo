//! The live harness warms every routed model before a run starts: the plan (which models, in
//! which order, none for a cassette) and the wait (ready, still loading, never there) over a
//! scripted inferd and a virtual clock.

use account_capability::Tier;
use action_accept::world::ModelSource;
use action_live::warm::{POLL, Prepare, WarmError, WarmStep, plan, warm_all};
use base_time::{ManualMonotonic, Sleeper};
use inference_core::Readiness;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::Duration;

/// An inferd that answers each ask from a script (`None` when it runs out: refused).
struct Scripted {
    answers: RefCell<VecDeque<Readiness>>,
    asked: RefCell<Vec<Tier>>,
}

/// A virtual clock that moves by the time it is asked to sleep, and does not wait.
#[derive(Default)]
struct Virtual(ManualMonotonic);

impl Sleeper for Virtual {
    async fn sleep(&self, how_long: Duration) {
        self.0.advance(how_long);
    }
}

impl Scripted {
    fn answering(answers: Vec<Readiness>) -> Self {
        Self {
            answers: RefCell::new(answers.into()),
            asked: RefCell::new(Vec::new()),
        }
    }
}

impl Prepare for Scripted {
    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn prepare(&self, tier: Tier) -> Option<Readiness> {
        self.asked.borrow_mut().push(tier);
        self.answers.borrow_mut().pop_front()
    }
}

fn live(text: &str) -> ModelSource {
    ModelSource::Live(text.to_owned())
}

fn step(tier: Tier, model: &str) -> WarmStep {
    WarmStep {
        tier,
        model: model.to_owned(),
    }
}

#[test]
fn a_cassette_has_nothing_to_warm() {
    assert_eq!(plan(&ModelSource::Scripted("{}".to_owned())), Ok(vec![]));
}

#[test]
fn each_distinct_model_is_asked_once_with_the_first_used_tier_last() {
    let config = r#"
[ai.model.text]
fast = "local/qwen"
balanced = "local/qwen"
best = "local/granite"
"#;
    assert_eq!(
        plan(&live(config)),
        Ok(vec![
            step(Tier::Best, "local/granite"),
            step(Tier::Fast, "local/qwen")
        ])
    );
    let three = "[ai.model.text]\nfast = \"a\"\nbalanced = \"b\"\nbest = \"c\"\n";
    let tiers: Vec<Tier> = plan(&live(three))
        .expect("plan")
        .iter()
        .map(|s| s.tier)
        .collect();
    assert_eq!(tiers, [Tier::Best, Tier::Balanced, Tier::Fast]);
    assert_eq!(plan(&live("[engines]\n")), Ok(vec![]));
    assert!(matches!(plan(&live("= nope")), Err(WarmError::Config(_))));
}

#[tokio::test]
async fn a_loading_model_is_waited_for_and_the_wait_is_reported() {
    let steps = [step(Tier::Best, "b"), step(Tier::Fast, "f")];
    let inferd = Scripted::answering(vec![
        Readiness::Loadable,
        Readiness::Loading,
        Readiness::Ready,
        Readiness::Ready,
    ]);
    let time = Virtual::default();
    let mut said = Vec::new();
    let warmed = warm_all(
        &steps,
        &inferd,
        (&time.0, &time),
        Duration::from_secs(60),
        |w| {
            said.push(w.line());
        },
    )
    .await
    .expect("warm");
    assert_eq!(warmed[0].took, POLL * 2);
    assert_eq!(warmed[1].took, Duration::ZERO);
    assert_eq!(
        *inferd.asked.borrow(),
        [Tier::Best, Tier::Best, Tier::Best, Tier::Fast]
    );
    assert_eq!(said.len(), 2);
    assert!(said[0].contains("b ready in 4.0s"), "{said:?}");
}

#[tokio::test]
async fn a_model_that_never_comes_up_stops_the_run_with_a_typed_fault() {
    let steps = [step(Tier::Best, "b"), step(Tier::Fast, "f")];
    let time = Virtual::default();
    let inferd = Scripted::answering(vec![Readiness::Loading; 10]);
    let fault = warm_all(
        &steps,
        &inferd,
        (&time.0, &time),
        Duration::from_secs(4),
        |_| {},
    )
    .await
    .expect_err("never ready");
    assert_eq!(
        fault,
        WarmError::NeverCameUp {
            step: step(Tier::Best, "b"),
            last: "loading",
            waited: POLL * 2,
        }
    );
    assert_eq!(
        inferd.asked.borrow().len(),
        3,
        "first ask, then one per poll"
    );
}

#[tokio::test]
async fn an_unavailable_or_refusing_inferd_fails_at_once() {
    let steps = [step(Tier::Fast, "f")];
    for answers in [vec![Readiness::Unavailable], vec![]] {
        let time = Virtual::default();
        let inferd = Scripted::answering(answers);
        let fault = warm_all(
            &steps,
            &inferd,
            (&time.0, &time),
            Duration::from_secs(60),
            |_| {},
        )
        .await
        .expect_err("unavailable");
        assert_eq!(fault, WarmError::Unavailable(step(Tier::Fast, "f")));
        assert_eq!(inferd.asked.borrow().len(), 1);
    }
}
