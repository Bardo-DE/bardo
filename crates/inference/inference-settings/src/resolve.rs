//! The file's `[ai]` table as [`Settings`].

use crate::AiConfig;
use crate::keys::{from_slug, parse_pool_key, slug_of};
use crate::{AGENT_ENDPOINT, DESCRIBE_IMAGES, LOCAL_ONLY, MY_NETWORK, TAILNET_SERVE, switch_of};
use crate::{
    LENGTH_AUTOMATIC, LENGTH_CONVERSATION, LENGTH_CONVERSATION_RANGE, LENGTH_REPLY,
    LENGTH_REPLY_RANGE, LengthSaid,
};
use crate::{
    SPEND_ACCOUNT_DAILY, SPEND_ACCOUNT_MONTHLY, SPEND_APP_DAILY, SPEND_APP_MONTHLY,
    SPEND_CAP_RANGE, SPEND_WARN, SPEND_WARN_DEFAULT, SPEND_WARN_RANGE, ScopeLimits, Settings,
    SpendLimits, SpendLine, cents_to_limit,
};
use account_capability::Tier;
use account_core::DataClass;
use base_ids::Permille;
use base_ids::Tokens;
use inference_core::{
    AutoRow, ClassFloor, Floor, ModelRef, Policy, PoolRow, Slot, TierMap, TierRow,
};
use inference_router::local::Lengths;

/// The settings in force, and the paths of the rows whose values were refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// What routing runs under.
    pub settings: Settings,
    /// The rows that held a value they do not accept and fell back to their default.
    pub rejected: Vec<String>,
}

/// The model `account/model` names.
#[must_use]
pub fn parse_model(text: &str) -> Option<ModelRef> {
    let (account, model) = text.split_once('/')?;
    Some(ModelRef {
        account: account_core::AccountId::parse(account).ok()?,
        model: account_capability::ModelId::parse(model).ok()?,
    })
}

/// `account/model`: what a model row says for a model.
#[must_use]
pub fn model_text(model: &ModelRef) -> String {
    format!("{}/{}", model.account, model.model)
}

fn policy_of(ai: &AiConfig, mut policy: Policy, rejected: &mut Vec<String>) -> Policy {
    policy.local_only = switch_of(
        ai.local_only.as_ref(),
        LOCAL_ONLY,
        policy.local_only,
        rejected,
    );
    for (class, word) in &ai.floor {
        let path = format!("ai.floor.{class}");
        match (from_slug::<DataClass>(class), from_slug::<Floor>(word)) {
            (Some(class), Some(floor)) => {
                match policy.floors.iter_mut().find(|row| row.class == class) {
                    Some(row) => row.floor = floor,
                    None => policy.floors.push(ClassFloor { class, floor }),
                }
            }
            _ => rejected.push(path),
        }
    }
    policy
}

/// The rows of `[ai.model.*]`.
fn model_rows(ai: &AiConfig) -> Vec<(&str, &str, &str)> {
    ai.model
        .iter()
        .flat_map(|(slot, tiers)| {
            tiers
                .iter()
                .map(move |(tier, text)| (slot.as_str(), tier.as_str(), text.as_str()))
        })
        .collect()
}

fn tiers_of(ai: &AiConfig, mut tiers: TierMap, rejected: &mut Vec<String>) -> TierMap {
    for (slot_slug, tier_slug, text) in model_rows(ai) {
        let path = format!("ai.model.{slot_slug}.{tier_slug}");
        let (Some(kind), Some(tier)) = (Slot::from_slug(slot_slug), from_slug::<Tier>(tier_slug))
        else {
            rejected.push(path);
            continue;
        };
        let slot = |row_kind: Slot, row_tier: Tier| row_kind == kind && row_tier == tier;
        let named = match text {
            "" | "auto" => None,
            other => match parse_model(other) {
                // A coding agent is a choice of the text slot alone.
                Some(model) if model.is_agent() && kind != Slot::Text => {
                    rejected.push(path);
                    continue;
                }
                Some(model) => Some(model),
                None => {
                    rejected.push(path);
                    continue;
                }
            },
        };
        // The row at its settings path replaces any earlier row for the slot.
        tiers.rows.retain(|r| !slot(r.kind, r.tier));
        tiers.autos.retain(|r| !slot(r.kind, r.tier));
        match (text, named) {
            (_, Some(model)) => tiers.rows.push(TierRow { kind, tier, model }),
            ("auto", None) => tiers.autos.push(AutoRow {
                kind,
                tier,
                mode: inference_core::AutoMode::default(),
            }),
            _ => {}
        }
    }
    for (key, texts) in &ai.auto.pool {
        let path = format!("ai.auto.pool.{key}");
        let Some((kind, chooser)) = parse_pool_key(key) else {
            rejected.push(path);
            continue;
        };
        // A row that names no model is dropped and said; the rest of the pool stands.
        // Automatic chooses among models and never hands a task to an agent, so an agent in the
        // pool is dropped and said the same way.
        let models: Vec<ModelRef> = texts
            .iter()
            .filter_map(|text| parse_model(text))
            .filter(|model| !model.is_agent())
            .collect();
        if models.len() != texts.len() {
            rejected.push(path);
        }
        tiers
            .pools
            .retain(|row| row.kind != kind || row.chooser != chooser);
        tiers.pools.push(PoolRow {
            kind,
            models,
            chooser,
        });
    }
    tiers
}

fn describe_images_of(ai: &AiConfig, rejected: &mut Vec<String>) -> bool {
    switch_of(
        ai.pipeline.describe_images.as_ref(),
        DESCRIBE_IMAGES,
        false,
        rejected,
    )
}

fn my_network_of(ai: &AiConfig, rejected: &mut Vec<String>) -> bool {
    switch_of(ai.attached.my_network.as_ref(), MY_NETWORK, false, rejected)
}

fn agent_endpoint_of(ai: &AiConfig, rejected: &mut Vec<String>) -> bool {
    switch_of(ai.agents.endpoint.as_ref(), AGENT_ENDPOINT, false, rejected)
}

fn tailnet_serve_of(ai: &AiConfig, rejected: &mut Vec<String>) -> bool {
    switch_of(ai.tailnet.serve.as_ref(), TAILNET_SERVE, false, rejected)
}

fn spend_of(ai: &AiConfig, rejected: &mut Vec<String>) -> SpendLine {
    let line = |permille: u32| Permille(permille);
    let warn_at = match ai.spend.warn_permille {
        None => line(SPEND_WARN_DEFAULT),
        Some(said) => match u32::try_from(said) {
            Ok(value) if SPEND_WARN_RANGE.contains(&value) => line(value),
            _ => {
                rejected.push(SPEND_WARN.to_owned());
                line(SPEND_WARN_DEFAULT)
            }
        },
    };
    let mut cap = |said: Option<i64>, path: &str| match said {
        None => None,
        Some(said) => match u32::try_from(said) {
            Ok(cents) if SPEND_CAP_RANGE.contains(&cents) => cents_to_limit(cents),
            _ => {
                rejected.push(path.to_owned());
                None
            }
        },
    };
    let limits = SpendLimits {
        account: ScopeLimits {
            daily: cap(ai.spend.account_daily_cents, SPEND_ACCOUNT_DAILY),
            monthly: cap(ai.spend.account_monthly_cents, SPEND_ACCOUNT_MONTHLY),
        },
        app: ScopeLimits {
            daily: cap(ai.spend.app_daily_cents, SPEND_APP_DAILY),
            monthly: cap(ai.spend.app_monthly_cents, SPEND_APP_MONTHLY),
        },
    };
    SpendLine { warn_at, limits }
}

/// A length row's value: none for Automatic or for a value out of `range` (said in `rejected`).
fn length_of(
    said: Option<&LengthSaid>,
    path: &str,
    range: &std::ops::RangeInclusive<u32>,
    rejected: &mut Vec<String>,
) -> Option<Tokens> {
    let number = match said? {
        LengthSaid::Words(word) if word == LENGTH_AUTOMATIC => return None,
        LengthSaid::Words(word) => word.parse::<u32>().ok(),
        LengthSaid::Number(number) => u32::try_from(*number).ok(),
    };
    match number {
        Some(value) if range.contains(&value) => Some(Tokens(value)),
        _ => {
            rejected.push(path.to_owned());
            None
        }
    }
}

fn lengths_of(ai: &AiConfig, rejected: &mut Vec<String>) -> Lengths {
    Lengths {
        conversation: length_of(
            ai.length.conversation.as_ref(),
            LENGTH_CONVERSATION,
            &LENGTH_CONVERSATION_RANGE,
            rejected,
        ),
        reply: length_of(
            ai.length.reply.as_ref(),
            LENGTH_REPLY,
            &LENGTH_REPLY_RANGE,
            rejected,
        ),
    }
}

/// The settings the `[ai]` table says.
#[must_use]
pub fn resolve(ai: &AiConfig) -> Resolved {
    let mut rejected = Vec::new();
    let auto = ai.auto.resolve();
    rejected.extend(auto.rejected.iter().map(|path| (*path).to_owned()));
    let settings = Settings {
        policy: policy_of(ai, Policy::proposed(), &mut rejected),
        tiers: tiers_of(ai, TierMap::default(), &mut rejected),
        auto: auto.policy,
        spend: spend_of(ai, &mut rejected),
        describe_images: describe_images_of(ai, &mut rejected),
        my_network: my_network_of(ai, &mut rejected),
        agent_endpoint: agent_endpoint_of(ai, &mut rejected),
        tailnet_serve: tailnet_serve_of(ai, &mut rejected),
        lengths: lengths_of(ai, &mut rejected),
    };
    Resolved { settings, rejected }
}

/// What the file says for one slot, as the picker's value: `""`, `auto` or `account/model`.
#[must_use]
pub fn slot_value(tiers: &TierMap, kind: Slot, tier: Tier) -> String {
    match tiers.pick(kind, tier) {
        None => String::new(),
        Some(inference_core::Pick::Named(model)) => model_text(&model),
        Some(inference_core::Pick::Auto { .. }) => "auto".to_owned(),
    }
}

/// A floor as the row's value.
#[must_use]
pub fn floor_value(policy: &Policy, class: DataClass) -> String {
    slug_of(&policy.floor(class))
}
