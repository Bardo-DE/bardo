//! The cascade, and the one place a verdict meets the rulings. A verdict can only make Cedar's
//! result stricter: `tighten` runs a call only from `AllowFinal`, or from `AllowJudged` when
//! every planned stage allowed it; a failure of any kind asks.

use crate::request::ReviewRequest;
use crate::verdict::ReviewVerdict;
use account_capability::ModelId;
use account_core::AccountId;
use permission_core::{DenyCode, Impact, ReviewError, Ruling, Stage};
use serde::{Deserialize, Serialize};

/// What a call may do after the cascade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Gate {
    /// Run it.
    Run,
    /// Ask the person (preview first).
    Confirm,
    /// Refuse it.
    Refuse(DenyCode),
}

/// The stages to run for a ruling. Only `AllowJudged` runs any: a low-impact call gets the
/// quick judge; a high-impact one (outbound or destructive, a count above half the mass
/// threshold, lasting memory) gets all three, the last from another model family.
#[must_use]
pub fn plan(ruling: &Ruling, impact: Impact) -> Vec<Stage> {
    match (ruling, impact) {
        (Ruling::AllowJudged(_), Impact::Low) => vec![Stage::Quick],
        (Ruling::AllowJudged(_), Impact::High) => {
            vec![Stage::Quick, Stage::Deliberate, Stage::SecondOpinion]
        }
        (Ruling::Deny(_) | Ruling::Ask(_) | Ruling::AllowFinal(_), _) => vec![],
    }
}

/// The plan after the quick judge flagged a low-impact call: the larger model looks too. The
/// planned stages still all have to allow, so a flag can ask or refuse but never run.
#[must_use]
pub fn escalate(planned: &[Stage], quick: &Result<ReviewVerdict, ReviewError>) -> Vec<Stage> {
    match (planned, quick) {
        ([Stage::Quick], Ok(ReviewVerdict::Ask { .. })) => vec![Stage::Quick, Stage::Deliberate],
        _ => planned.to_vec(),
    }
}

/// Combines a ruling with the verdicts of the planned stages. Total and pure.
///
/// `Deny`, `Ask` and `AllowFinal` pass through untouched whatever the verdicts are. For
/// `AllowJudged`: any `Deny` refuses; every planned stage must have a verdict, and every one
/// must be `Ok(Allow)`, to run; a missing verdict, an error or an `Ask` confirms. An empty plan
/// never runs.
#[must_use]
pub fn tighten(
    ruling: &Ruling,
    planned: &[Stage],
    verdicts: &[(Stage, Result<ReviewVerdict, ReviewError>)],
) -> Gate {
    match ruling {
        Ruling::Deny(_) => Gate::Refuse(DenyCode::NotAllowed),
        Ruling::Ask(_) => Gate::Confirm,
        Ruling::AllowFinal(_) => Gate::Run,
        Ruling::AllowJudged(_) => {
            let denied = verdicts
                .iter()
                .any(|(_, v)| matches!(v, Ok(ReviewVerdict::Deny { .. })));
            let allowed = |stage: &Stage| {
                verdicts
                    .iter()
                    .any(|(s, v)| s == stage && matches!(v, Ok(ReviewVerdict::Allow)))
            };
            if denied {
                Gate::Refuse(DenyCode::NotAllowed)
            } else if !planned.is_empty() && planned.iter().all(allowed) {
                Gate::Run
            } else {
                Gate::Confirm
            }
        }
    }
}

/// What a stage's reviewer implements.
pub trait Reviewer: Send + Sync {
    /// Reviews a stripped request at one stage.
    fn review(
        &self,
        stage: Stage,
        request: &ReviewRequest,
    ) -> impl Future<Output = Result<ReviewVerdict, ReviewError>> + Send;
}

/// One model chosen for a stage.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelChoice {
    /// The account that serves it.
    pub account: AccountId,
    /// The model.
    pub model: ModelId,
    /// Its family.
    pub family: String,
}

/// The three models behind the stages.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReviewerSet {
    /// A small local judge.
    pub quick: ModelChoice,
    /// A larger local model.
    pub deliberate: ModelChoice,
    /// A model of another family.
    pub second: ModelChoice,
}

impl ReviewerSet {
    /// Whether the second opinion is a different family from the deliberate model. Without
    /// one, high-impact allows fall back to asking.
    #[must_use]
    pub fn second_is_other_family(&self) -> bool {
        self.second.family != self.deliberate.family
    }
}
