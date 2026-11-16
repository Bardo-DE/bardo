//! bardo-shell as an `IntentProvider1`: the shell's own actions. intentd calls `Perform`; the provider
//! reads the invocation into a [`ShellAction`], has the daemon do it (a [`ShellCall`]) and says
//! how it went. Everything else a provider can be asked is answered with nothing to say: the
//! shell indexes nothing, previews nothing and declares no `dry_run`.

use std::time::Duration;

use action_client::DbusTransport;
use action_core::{
    ActivationToken, AppRefusal, ContextSnapshot, FailText, Here, Hit, Invocation, Outcome,
    Preview, Selection, SuggestAsk, TextTarget, UndoFault, Visible, WindowPrivacy,
};
use action_manifest::{Manifest, ValidManifest, validate};
use action_provider::{ContextSource, IntentProvider, SummonTarget};
use action_vocab::{CallClass, EntityRef, UndoToken};
use action_wire::{SummonAnswer, SummonOrigin, SummonSerial};
use base_ids::Count;
use provenance::{Actor, EntityId, Label, Labelled};
use shell_model::intents::shell::{
    ShellAction, ShellFault, action_of, menu_ask_of, outcome_of, undo_of, window_ask_of,
};
use shell_model::screenshots::Shots;
use tokio::sync::{mpsc, oneshot, watch};

use crate::app_menu::{MenuAgent, MenuLinks};
use crate::intents::ShellCall;

/// What the shell declares, as shipped: the installed file, held to the model by a test.
const MANIFEST: &str = include_str!("../../../../dist/intents/org.bardo.Shell.toml");

/// How long the daemon has to do one action (the manifest's `quick` latency is five seconds).
const CALL_WAIT: Duration = Duration::from_secs(4);

/// The shell's manifest, parsed and validated; an error is a bug in the shipped file.
///
/// # Errors
/// [`ManifestFault::Parse`] when the shipped file is not TOML of the manifest's shape,
/// [`ManifestFault::Invalid`] when it breaks a rule of the vocabulary.
pub fn manifest() -> Result<ValidManifest, ManifestFault> {
    let parsed: Manifest = toml::from_str(MANIFEST)?;
    Ok(validate(parsed)?)
}

/// Why the shipped manifest cannot be read: a bug in the shipped file.
#[derive(Debug, thiserror::Error)]
pub enum ManifestFault {
    /// The file is not TOML of the manifest's shape.
    #[error("{0}")]
    Parse(#[from] toml::de::Error),
    /// The manifest parses but breaks a rule of the vocabulary.
    #[error("{0}")]
    Invalid(#[from] action_manifest::ManifestError),
}

/// The shell's provider.
pub(super) struct ShellProvider {
    pub manifest: ValidManifest,
    pub calls: mpsc::UnboundedSender<ShellCall>,
    /// The focused window's menus, for `shell.menu.list` and `shell.menu.activate`.
    pub menus: MenuAgent<DbusTransport>,
    /// The windows and the compositor, for the window and workspace actions.
    pub windows: MenuLinks,
    /// The screenshots the shell has seen, for the path `shell.screenshot` answers with.
    pub shots: watch::Receiver<Shots>,
}

impl ShellProvider {
    /// Have the daemon do `action` and wait for how it went.
    async fn done(&self, action: ShellAction) -> Result<(), ShellFault> {
        let (reply, answer) = oneshot::channel();
        self.calls
            .send(ShellCall { action, reply })
            .map_err(|e| ShellFault::Failed(format!("the shell is shutting down: {e}")))?;
        match tokio::time::timeout(CALL_WAIT, answer).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(ShellFault::Failed("the shell did not answer".to_owned())),
            Err(_) => Err(ShellFault::Failed("the shell took too long".to_owned())),
        }
    }
}

fn refusal(fault: ShellFault) -> AppRefusal {
    match fault {
        ShellFault::Unsupported => AppRefusal::Unsupported,
        ShellFault::Failed(why) => AppRefusal::Failed(FailText(why)),
    }
}

impl IntentProvider for ShellProvider {
    fn manifest(&self) -> &ValidManifest {
        &self.manifest
    }

    async fn perform(&self, inv: Invocation) -> Result<Outcome, AppRefusal> {
        self.perform_classified(inv, None, None).await
    }

    async fn perform_classified(
        &self,
        inv: Invocation,
        activation: Option<ActivationToken>,
        classified: Option<CallClass>,
    ) -> Result<Outcome, AppRefusal> {
        let _ = activation;
        if let Some(ask) = menu_ask_of(&inv)? {
            return super::menus::perform(&self.menus, ask, &inv, classified).await;
        }
        if let Some(ask) = window_ask_of(&inv)? {
            return super::windows::perform(&self.windows, &ask, &inv);
        }
        let action = action_of(&inv)?;
        let first = self.shots.borrow().last_id;
        self.done(action.clone()).await.map_err(refusal)?;
        let mut outcome = outcome_of(&action);
        if let ShellAction::Screenshot(_) = action {
            outcome.value = super::shots::saved(&mut self.shots.clone(), first, &inv).await;
        }
        Ok(outcome)
    }

    async fn classify(&self, inv: Invocation) -> Result<CallClass, AppRefusal> {
        match menu_ask_of(&inv)? {
            Some(ask) => super::menus::classify(&self.menus, ask).await,
            None => Err(AppRefusal::Unsupported),
        }
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the provider trait's method is async; this one has nothing to wait on"
    )]
    async fn dry_run(&self, inv: Invocation) -> Result<Preview, AppRefusal> {
        if menu_ask_of(&inv)?.is_some() || window_ask_of(&inv)?.is_some() {
            return Ok(Preview::None);
        }
        action_of(&inv).map(|_| Preview::None)
    }

    async fn undo(&self, token: UndoToken, actor: Actor) -> Result<(), UndoFault> {
        let _ = actor;
        let action = undo_of(&token)?;
        #[expect(
            clippy::map_err_ignore,
            reason = "any failure of the undo's action means the same: what it undid is gone"
        )]
        self.done(action).await.map_err(|_| UndoFault::Gone)
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the provider trait's method is async; this one has nothing to wait on"
    )]
    async fn search(&self, text: &str) -> Vec<Hit> {
        let _ = text;
        Vec::new()
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the provider trait's method is async; this one has nothing to wait on"
    )]
    async fn preview(&self, id: &EntityId) -> Preview {
        let _ = id;
        Preview::None
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the provider trait's method is async; this one has nothing to wait on"
    )]
    async fn suggest(&self, ask: SuggestAsk) -> Vec<EntityRef> {
        let _ = ask;
        Vec::new()
    }
}

/// What the shell shows the router when asked "what is the person looking at": nothing in
/// particular, in the shell.
pub(super) struct ShellContext {
    pub app: provenance::AppName,
}

impl ContextSource for ShellContext {
    fn snapshot(&self) -> ContextSnapshot {
        ContextSnapshot {
            app: self.app.clone(),
            window: Labelled {
                value: "bardo-shell".to_owned(),
                label: Label::trusted_user(),
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

/// The shell has no prompt field to summon the companion into.
pub(super) struct NoSummon;

impl SummonTarget for NoSummon {
    fn summon(&self, serial: SummonSerial, origin: SummonOrigin) -> SummonAnswer {
        let _ = (serial, origin);
        SummonAnswer::Declined
    }
}
