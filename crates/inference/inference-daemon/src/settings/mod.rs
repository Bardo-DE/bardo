//! The settings inferd serves and reads (design/22 section 5, the Intelligence page): the rows of
//! `dist/inferd.settings.toml` and the `ai.model.<slot>.<tier>` picker, a live module.
//!
//! * [`Settings`] is what routing runs under: the policy (`ai.local_only`, `ai.floor.<class>`), the
//!   tier map (`ai.model.<slot>.<tier>`), Automatic (`ai.auto.*`) and the spend line
//!   (`ai.spend.warn_permille`) and, for the models on this computer, the longest conversation
//!   and reply the person set (`ai.length.*`, Automatic where they set none; a window takes effect
//!   the next time an engine starts). [`resolve`] reads it from the file's `[ai]` table; a value a row does not accept falls back to the row's
//!   default, for that field only, and is named in [`Resolved::rejected`].
//! * [`Live`] holds the settings in force, shared by every clone of `Engines`; a session is routed
//!   by what is in force when it opens.
//! * [`ConfigFile`] reads and edits the file, and [`Reload`] puts what it says in force again: the
//!   daemon polls the file's mtime, `Rescan` reloads at once, and a `Set` of the module applies
//!   what it wrote.
//! * [`InferdSettings`] is `org.quire.SettingsModule1` at `INFERENCE_SETTINGS_PATH`.

mod file;
mod keys;
mod local_engines;
mod missing;
mod module;
mod pool;
mod programs;
mod resolve;

pub use file::{ConfigFile, FileError, Reload, ReloadFailed, Reloaded};
pub use keys::{CLASSES, SLOTS, TIERS, from_slug, model_path, slug_of};
pub use local_engines::LocalEngines;
pub(crate) use missing::engine_name;
pub use module::{InferdSettings, serve_settings};
pub use pool::HelperPools;
pub use resolve::{Resolved, floor_value, model_text, parse_model, resolve, slot_value};

use account_core::AccountId;
use account_core::AppId;
use account_ids::Permille;
use inference_core::{
    AutoPolicy, ClassFloor, Floor, Period, Policy, SpendCap, SpendScope, TierMap,
};
use inference_router::local::Lengths;
use model_provider::Money;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::watch;

/// `ai.local_only`.
pub const LOCAL_ONLY: &str = "ai.local_only";

/// `ai.attached.my_network`.
pub const MY_NETWORK: &str = "ai.attached.my_network";

/// A switch row as written: `true` or `false`, or a word, which the row refuses (an old file
/// that spelled a switch `"on"`), so that one row falls back to its default and the rest of the
/// file stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SwitchSaid {
    /// `true` or `false`.
    Switch(bool),
    /// Any word.
    Word(String),
}

/// The switch row at `path`: `default` where the file says nothing, and also where it says a word
/// (said in `rejected`).
pub(crate) fn switch_of(
    said: Option<&SwitchSaid>,
    path: &str,
    default: bool,
    rejected: &mut Vec<String>,
) -> bool {
    match said {
        None => default,
        Some(SwitchSaid::Switch(on)) => *on,
        Some(SwitchSaid::Word(_)) => {
            rejected.push(path.to_owned());
            default
        }
    }
}

/// The `[ai.attached]` table, as written.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachedConfig {
    /// `ai.attached.my_network`: whether an engine the person attached on another machine of
    /// theirs (`where = "my-network"`) counts as this computer for a data class whose floor is
    /// "this computer". Off by default: such data goes only to this computer. While on, an
    /// on-device floor admits it (a floor that already says "my machines" admits it either way).
    #[serde(default)]
    pub my_network: Option<SwitchSaid>,
}

/// `ai.agents.endpoint`.
pub const AGENT_ENDPOINT: &str = "ai.agents.endpoint";

/// The `[ai.agents]` table, as written.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentsConfig {
    /// `ai.agents.endpoint`: whether inferd opens loopback model endpoints for the external
    /// coding agents the launcher starts. Off by default, like the other ways in for agents (ACP,
    /// MCP); off, `OpenEndpoint` is refused `EndpointOff`.
    #[serde(default)]
    pub endpoint: Option<SwitchSaid>,
}

/// `ai.tailnet.serve`.
pub const TAILNET_SERVE: &str = "ai.tailnet.serve";

/// The `[ai.tailnet]` table, as written.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TailnetConfig {
    /// `ai.tailnet.serve`: whether this computer lends its own models to the person's other
    /// computers over their Tailscale network. Off by default; while on, inferd listens on this
    /// computer's network addresses (never on every address), and a computer is let in only when
    /// Tailscale says it is the person's own and the person said yes to it.
    #[serde(default)]
    pub serve: Option<SwitchSaid>,
}

/// `ai.pipeline.describe_images`.
pub const DESCRIBE_IMAGES: &str = "ai.pipeline.describe_images";

/// `ai.length.conversation`.
pub const LENGTH_CONVERSATION: &str = "ai.length.conversation";
/// `ai.length.reply`.
pub const LENGTH_REPLY: &str = "ai.length.reply";
/// What the two length rows say for "worked out for each model".
pub const LENGTH_AUTOMATIC: &str = "automatic";
/// The conversation lengths the row accepts, in the units models count by: from about 3 pages to
/// about 1600 (a page is about 650 of them).
pub const LENGTH_CONVERSATION_RANGE: std::ops::RangeInclusive<u32> = 2048..=1_048_576;
/// The reply lengths the row accepts: from about half a page to about 400.
pub const LENGTH_REPLY_RANGE: std::ops::RangeInclusive<u32> = 256..=262_144;

/// A length row as written: a word (`automatic`, or a number in quotes, as Settings writes it) or
/// a bare whole number. Signed, so a negative number reaches the range check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LengthSaid {
    /// `65536`.
    Number(i64),
    /// `"automatic"` or `"65536"`.
    Words(String),
}

/// The `[ai.length]` table, as written.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LengthConfig {
    /// `ai.length.conversation`.
    #[serde(default)]
    pub conversation: Option<LengthSaid>,
    /// `ai.length.reply`.
    #[serde(default)]
    pub reply: Option<LengthSaid>,
}

/// `ai.spend.warn_permille`.
pub const SPEND_WARN: &str = "ai.spend.warn_permille";
/// What the row says without a file, in thousandths of a cap.
pub const SPEND_WARN_DEFAULT: u32 = 800;
/// The values the row accepts.
pub const SPEND_WARN_RANGE: std::ops::RangeInclusive<u32> = 1..=1000;

/// The four cap rows, in whole US cents; `0` is no cap. A cap on an account counts everything
/// spent through it, whoever spent it; a cap on an app counts everything it spends, whichever
/// account paid.
pub const SPEND_ACCOUNT_DAILY: &str = "ai.spend.account_daily_cents";
/// `ai.spend.account_monthly_cents`.
pub const SPEND_ACCOUNT_MONTHLY: &str = "ai.spend.account_monthly_cents";
/// `ai.spend.app_daily_cents`.
pub const SPEND_APP_DAILY: &str = "ai.spend.app_daily_cents";
/// `ai.spend.app_monthly_cents`.
pub const SPEND_APP_MONTHLY: &str = "ai.spend.app_monthly_cents";
/// The values the cap rows accept: up to a hundred thousand dollars.
pub const SPEND_CAP_RANGE: std::ops::RangeInclusive<u32> = 0..=10_000_000;

/// The `[ai.spend]` table, as written. Signed, so a negative number reaches the range check.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpendConfig {
    /// `ai.spend.warn_permille`.
    #[serde(default)]
    pub warn_permille: Option<i64>,
    /// `ai.spend.account_daily_cents`.
    #[serde(default)]
    pub account_daily_cents: Option<i64>,
    /// `ai.spend.account_monthly_cents`.
    #[serde(default)]
    pub account_monthly_cents: Option<i64>,
    /// `ai.spend.app_daily_cents`.
    #[serde(default)]
    pub app_daily_cents: Option<i64>,
    /// `ai.spend.app_monthly_cents`.
    #[serde(default)]
    pub app_monthly_cents: Option<i64>,
}

/// The `[ai.pipeline]` table, as written.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// `ai.pipeline.describe_images`.
    #[serde(default)]
    pub describe_images: Option<SwitchSaid>,
}

/// One scope's limits: none where the person set none.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScopeLimits {
    /// Per calendar day.
    pub daily: Option<Money>,
    /// Per calendar month.
    pub monthly: Option<Money>,
}

/// The caps the person set: on each account, and on each app.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpendLimits {
    /// Per account.
    pub account: ScopeLimits,
    /// Per app.
    pub app: ScopeLimits,
}

/// The share of a cap at which a request is warned about, and the caps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpendLine {
    /// `ai.spend.warn_permille`.
    pub warn_at: Permille,
    /// `ai.spend.{account,app}_{daily,monthly}_cents`.
    pub limits: SpendLimits,
}

impl Default for SpendLine {
    fn default() -> Self {
        Self {
            warn_at: Permille(SPEND_WARN_DEFAULT),
            limits: SpendLimits::default(),
        }
    }
}

/// A cap row's value as an amount of US dollars; `None` for no cap (`0`).
pub fn cents_to_limit(cents: u32) -> Option<Money> {
    (cents > 0).then(|| Money::usd(Decimal::new(i64::from(cents), 2)))
}

impl SpendLine {
    /// A cap on `scope` over `period` with this line as its warning: the one place inferd makes a
    /// [`SpendCap`], so `SpendCap.warn_at` is always the row's value.
    pub fn cap(self, scope: SpendScope, period: Period, limit: Money) -> SpendCap {
        SpendCap {
            scope,
            period,
            limit,
            warn_at: self.warn_at,
        }
    }

    /// The caps that bear on one more request by `app` through `account`.
    pub fn caps_for(self, app: &AppId, account: &AccountId) -> Vec<SpendCap> {
        let SpendLimits {
            account: on_account,
            app: on_app,
        } = self.limits;
        let rows = [
            (
                SpendScope::Account(account.clone()),
                Period::Daily,
                on_account.daily,
            ),
            (
                SpendScope::Account(account.clone()),
                Period::Monthly,
                on_account.monthly,
            ),
            (SpendScope::App(app.clone()), Period::Daily, on_app.daily),
            (
                SpendScope::App(app.clone()),
                Period::Monthly,
                on_app.monthly,
            ),
        ];
        rows.into_iter()
            .filter_map(|(scope, period, limit)| Some(self.cap(scope, period, limit?)))
            .collect()
    }
}

/// What routing runs under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// `ai.local_only` and `ai.floor.<class>`.
    pub policy: Policy,
    /// `ai.model.<slot>.<tier>`.
    pub tiers: TierMap,
    /// `ai.auto.*`.
    pub auto: AutoPolicy,
    /// `ai.spend.*`.
    pub spend: SpendLine,
    /// `ai.pipeline.describe_images`.
    pub describe_images: bool,
    /// `ai.attached.my_network`.
    pub my_network: bool,
    /// `ai.agents.endpoint`.
    pub agent_endpoint: bool,
    /// `ai.tailnet.serve`.
    pub tailnet_serve: bool,
    /// `ai.length.*`: what the person set for the models on this computer; Automatic where not.
    pub lengths: Lengths,
}

impl Settings {
    /// The policy routing runs under: `policy`, with every floor that says "this computer" let
    /// reach the person's other machines when `ai.attached.my_network` is on. `local_only` and the
    /// floors that already allow more are as the person set them.
    pub fn routing_policy(&self) -> Policy {
        if !self.my_network {
            return self.policy.clone();
        }
        Policy {
            floors: self
                .policy
                .floors
                .iter()
                .map(|row| ClassFloor {
                    floor: row.floor.max(Floor::LocalNetwork),
                    ..*row
                })
                .collect(),
            ..self.policy.clone()
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            policy: Policy::proposed(),
            tiers: TierMap::default(),
            auto: AutoPolicy::default(),
            spend: SpendLine::default(),
            describe_images: false,
            my_network: false,
            agent_endpoint: false,
            tailnet_serve: false,
            lengths: Lengths::default(),
        }
    }
}

/// The settings in force: replaced whole, read as a snapshot, and followed by whoever
/// [`Live::subscribe`]s.
#[derive(Debug, Clone)]
pub struct Live(Arc<watch::Sender<Arc<Settings>>>);

impl Default for Live {
    fn default() -> Self {
        Self::new(Settings::default())
    }
}

impl Live {
    /// A holder of `settings`.
    pub fn new(settings: Settings) -> Self {
        Self(Arc::new(watch::Sender::new(Arc::new(settings))))
    }

    /// What is in force.
    pub fn get(&self) -> Arc<Settings> {
        Arc::clone(&self.0.borrow())
    }

    /// Puts `settings` in force.
    pub fn set(&self, settings: Settings) {
        self.0.send_replace(Arc::new(settings));
    }

    /// Follows the settings: the receiver changes whenever they are put in force again (also
    /// when nothing differs).
    pub fn subscribe(&self) -> watch::Receiver<Arc<Settings>> {
        self.0.subscribe()
    }
}

#[cfg(test)]
mod my_network_tests;
#[cfg(test)]
mod tests;
