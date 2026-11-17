//! Your computers: the person's computers over inferd, the Add list, the guests, and the model
//! servers added by hand. The steps are `settings-app-model`'s `computers_step`.

use crate::live::Live;
use crate::notice::Notice;
use crate::pane::{Machine, Pending, pane};
use settings_app_link::computers::{Change, ChangeFailure, ComputersLink};
use settings_app_model::{
    ComputersEffect, ComputersInput, ComputersState, ServerRefusal, computers_step,
};
use std::sync::Arc;

/// The computers' handle: cheap to clone, one live value behind it.
#[derive(Debug, Clone)]
pub struct Computers {
    link: Arc<dyn ComputersLink>,
    live: Live<ComputersState>,
}

pane!(Computers, ComputersState, follow);

impl Computers {
    pub(crate) fn over(link: Arc<dyn ComputersLink>) -> Computers {
        Computers {
            link,
            live: Live::default(),
        }
    }

    /// Gives the pane `input` now; the returned work carries out what it asks for.
    #[must_use]
    pub fn send(&self, input: ComputersInput) -> Pending {
        self.dispatch(input)
    }

    /// Reads the lists, then reads them again each time inferd says they changed or comes back.
    async fn follow(&self) {
        let mut owners = match self.link.owner_changes().await {
            Ok(owners) => owners,
            Err(absence) => return self.dispatch(ComputersInput::Found(Err(absence))).await,
        };
        loop {
            // Subscribed before the first read, so a change in between is not lost.
            let watch = self.link.changes().await;
            self.reread().await;
            if let Ok(mut watch) = watch {
                while watch.recv().await.is_some() {
                    self.reread().await;
                }
            }
            if owners.recv().await.is_none() {
                return;
            }
        }
    }

    async fn reread(&self) {
        self.dispatch(ComputersInput::Found(self.link.read().await))
            .await;
    }

    async fn changed(&self, change: Change) -> ComputersInput {
        let ended = self.link.change(change).await.map_err(refusal_of);
        ComputersInput::Changed(ended)
    }
}

/// Why a change was not made: the form's own fault, or the service's reason.
#[expect(
    clippy::needless_pass_by_value,
    reason = "used as a `map_err` function, which hands the error over by value"
)]
fn refusal_of(failure: ChangeFailure) -> ServerRefusal {
    match failure {
        ChangeFailure::Fault(fault) => ServerRefusal::Form(fault),
        ChangeFailure::Refused(reason) => ServerRefusal::Service(reason),
    }
}

impl Machine for Computers {
    type Input = ComputersInput;
    type Effect = ComputersEffect;

    fn step(
        state: ComputersState,
        input: ComputersInput,
    ) -> (ComputersState, Vec<ComputersEffect>) {
        computers_step(state, input)
    }

    async fn run(&self, effect: ComputersEffect) -> Vec<ComputersInput> {
        let input = match effect {
            ComputersEffect::Tell(reason) => {
                self.live
                    .notice(Notice::ComputerRefused(ServerRefusal::Service(reason)));
                return Vec::new();
            }
            ComputersEffect::Refused(refusal) => {
                self.live.notice(Notice::ComputerRefused(refusal));
                return Vec::new();
            }
            ComputersEffect::Reload => ComputersInput::Found(self.link.read().await),
            ComputersEffect::LoadCandidates => {
                ComputersInput::Candidates(self.link.candidates().await)
            }
            ComputersEffect::LoadServerModels => {
                ComputersInput::ServerModels(self.link.models().await)
            }
            ComputersEffect::Add(node) => self.changed(Change::Add(node)).await,
            ComputersEffect::Remove(id) => self.changed(Change::Remove(id)).await,
            ComputersEffect::Answer(node, answer) => {
                self.changed(Change::Answer(node, answer)).await
            }
            ComputersEffect::Forget(node) => self.changed(Change::Forget(node)).await,
            ComputersEffect::AddServer(draft) => {
                let ended = self
                    .link
                    .change(Change::Server(draft))
                    .await
                    .map_err(refusal_of);
                ComputersInput::ServerAdded(ended)
            }
        };
        vec![input]
    }
}
