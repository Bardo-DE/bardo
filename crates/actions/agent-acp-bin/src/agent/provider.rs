//! The host's side of the `org.bardo.AcpAgent` pseudo-app on the bus: intentd calls
//! `org.bardo.IntentProvider1` here after the router allowed an agent's call, and the performer
//! does it. Only intentd may call a provider (`action-client` enforces it), so nothing reaches
//! the performer that the gate did not allow.

use account_core::AppName;
use action_core::{
    AppRefusal, ContextSnapshot, Here, Hit, Invocation, Outcome, Preview, Selection, SuggestAsk,
    TextTarget, UndoFault, Visible, WindowPrivacy,
};
use action_manifest::ValidManifest;
use action_provider::{ContextSource, IntentProvider, SummonTarget};
use action_vocab::{EntityRef, UndoToken};
use action_wire::{SummonAnswer, SummonOrigin, SummonSerial};
use agent_acp_host::{Files, Performer};
use base_ids::Count;
use provenance::{Actor, Confidentiality, EntityId, Integrity, Label, Labelled, Source};
use sandbox::Sandbox;
use std::collections::BTreeSet;

/// The performer, as an app's provider.
pub struct PerformerProvider<F: Files + 'static, S: Sandbox + 'static> {
    manifest: ValidManifest,
    performer: Performer<F, S>,
}

impl<F: Files + 'static, S: Sandbox + 'static> std::fmt::Debug for PerformerProvider<F, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PerformerProvider")
    }
}

impl<F: Files + 'static, S: Sandbox + 'static> PerformerProvider<F, S> {
    /// Serves `performer` under `manifest` (`manifests/org.bardo.AcpAgent.toml`).
    pub fn new(manifest: ValidManifest, performer: Performer<F, S>) -> Self {
        Self {
            manifest,
            performer,
        }
    }
}

impl<F: Files + 'static, S: Sandbox + 'static> IntentProvider for PerformerProvider<F, S> {
    fn manifest(&self) -> &ValidManifest {
        &self.manifest
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn perform(&self, inv: Invocation) -> Result<Outcome, AppRefusal> {
        self.performer.perform(&inv)
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn dry_run(&self, _inv: Invocation) -> Result<Preview, AppRefusal> {
        Err(AppRefusal::Unsupported)
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn undo(&self, token: UndoToken, _actor: Actor) -> Result<(), UndoFault> {
        self.performer.undo(&token)
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn search(&self, _text: &str) -> Vec<Hit> {
        Vec::new()
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn preview(&self, _id: &EntityId) -> Preview {
        Preview::None
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn suggest(&self, _ask: SuggestAsk) -> Vec<EntityRef> {
        Vec::new()
    }
}

/// The host has no window: its context is a quiet one, and it takes no summon.
#[derive(Debug, Clone)]
pub struct Quiet(pub AppName);

impl ContextSource for Quiet {
    fn snapshot(&self) -> ContextSnapshot {
        ContextSnapshot {
            app: self.0.clone(),
            window: Labelled {
                value: "Agent".to_owned(),
                label: Label {
                    integrity: Integrity::Trusted,
                    confidentiality: Confidentiality::Public,
                    classes: BTreeSet::new(),
                    sources: BTreeSet::from([Source::App(self.0.clone())]),
                },
            },
            here: Here::Nowhere,
            selection: Selection::Nothing,
            visible: Visible {
                kind: None,
                items: Vec::new(),
                total: Count(0),
            },
            text_target: TextTarget::None,
            privacy: WindowPrivacy::Normal,
        }
    }
}

impl SummonTarget for Quiet {
    fn summon(&self, _serial: SummonSerial, _origin: SummonOrigin) -> SummonAnswer {
        SummonAnswer::Declined
    }
}
