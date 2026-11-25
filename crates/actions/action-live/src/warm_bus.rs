//! The shipped [`Prepare`]: inference-client's `Transport::prepare` over a private bus, as the
//! actiond unit (the policy writer's identity, which inferd serves every tier of a run to).

use crate::warm::{POLL, Prepare, WarmError, plan, warm_all};
use account_capability::LlmFeature;
use account_capability::Tier;
use account_core::DataClass;
use account_core::consent::Usage;
use account_match::LlmNeed;
use account_match::Need;
use action_accept::world::{Cgroup, ModelSource, World, place, restore_unit, unit_main};
use base_ids::Tokens;
use inference_client::{DbusTransport, OpenOptions, Transport};
use inference_core::Readiness;
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

/// Asks inferd over `connection`.
#[derive(Debug)]
pub struct BusPrepare {
    transport: DbusTransport,
    began: Instant,
}

impl BusPrepare {
    /// Prepares over `connection`.
    #[must_use]
    pub fn over(connection: zbus::Connection) -> Self {
        Self {
            transport: DbusTransport::over(connection),
            began: Instant::now(),
        }
    }
}

/// What a tier's callers ask of their model: the planner (balanced) calls tools, the policy
/// writer and the reviewers (fast, best) want structured output. Asking every tier for both would
/// call a planner that has no structured output unavailable, though it serves the planner.
fn need(tier: Tier) -> Need {
    let feature = match tier {
        Tier::Balanced => LlmFeature::Tools,
        _ => LlmFeature::StructuredOutput,
    };
    Need::Llm(LlmNeed::new(
        BTreeSet::from([LlmFeature::Chat, feature]),
        Tokens(2_000),
    ))
}

impl Prepare for BusPrepare {
    async fn prepare(&self, tier: Tier) -> Option<Readiness> {
        self.transport
            .prepare(
                &need(tier),
                DataClass::Prompt,
                tier,
                &OpenOptions::default().with_usage(Usage::Interactive),
            )
            .await
            .ok()
    }

    async fn pause(&self) {
        tokio::time::sleep(POLL).await;
    }

    fn now(&self) -> Duration {
        self.began.elapsed()
    }
}

/// Warms every model `source` routes to over `connection` (a cassette routes to none, and this
/// does nothing), printing each as it is ready. The caller is placed as actiond.
///
/// # Errors
///
/// Returns a [`WarmError`] when a model does not come up.
pub async fn warm_up(
    source: &ModelSource,
    connection: zbus::Connection,
    patience: Duration,
) -> Result<(), WarmError> {
    let steps = plan(source)?;
    let inferd = BusPrepare::over(connection);
    warm_all(&steps, &inferd, patience, |w| eprintln!("{}", w.line()))
        .await
        .map(drop)
}

/// Runs `work` with the harness process standing in as the actiond unit, then puts both back.
///
/// Runs `work` with the harness process standing in as the actiond unit, then puts both back:
/// the harness in the shell's unit, and the unit's main pid with the daemon that really is
/// actiond (a service caller is known by its unit's main pid, so leaving the harness there would
/// make memoryd refuse every record of the real actiond).
pub async fn as_actiond<T>(world: &World, work: impl std::future::Future<Output = T>) -> T {
    let (root, pid) = (world.dir.path(), std::process::id());
    let daemon = unit_main(root, "bardo-actiond");
    place(root, pid, Cgroup::Unit("bardo-actiond"));
    let done = work.await;
    place(root, pid, Cgroup::Unit("bardo-shell"));
    restore_unit(root, "bardo-actiond", daemon);
    done
}

/// Warms a smoke world's models. The harness process stands in for the shell there; for the warm-up
/// it is placed as actiond (the identity inferd serves a run's models to) and put back after.
///
/// # Errors
///
/// Returns a [`WarmError`] when a model does not come up.
pub async fn warm_world(
    world: &World,
    source: &ModelSource,
    patience: Duration,
) -> Result<(), WarmError> {
    if matches!(source, ModelSource::Scripted(_)) {
        return Ok(());
    }
    let connection = world.connect().await;
    as_actiond(world, warm_up(source, connection, patience)).await
}
