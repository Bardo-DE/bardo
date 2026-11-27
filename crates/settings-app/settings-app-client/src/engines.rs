//! The llama.cpp note under "Models on this computer": looked for when the pane appears and again
//! each time the assistant's name changes hands, never starting it, and the assistant's restart.
//! The steps are `settings-app-model`'s `engines_step`.

mod programs;

pub use programs::EnginePrograms;

use crate::live::Live;
use crate::notice::Notice;
use crate::pane::{Machine, Pending, pane};
use settings_app_link::engines::EnginesLink;
use settings_app_model::{EnginesEffect, EnginesInput, EnginesState, LlamaCpp, engines_step};
use std::sync::Arc;

/// The note's handle: cheap to clone, one live value behind it.
#[derive(Debug, Clone)]
pub struct Engines {
    link: Arc<dyn EnginesLink>,
    live: Live<EnginesState>,
}

pane!(Engines, EnginesState, follow);

impl Engines {
    pub(crate) fn over(link: Arc<dyn EnginesLink>) -> Engines {
        Engines {
            link,
            live: Live::default(),
        }
    }

    /// Gives the pane `input` now; the returned work carries out what it asks for.
    pub fn send(&self, input: EnginesInput) -> Pending {
        self.dispatch(input)
    }

    /// Looks when the pane appears and again each time the assistant's name changes hands.
    async fn follow(&self) {
        // Subscribed before the first look, so a change in between is not lost.
        let Ok(mut owners) = self.link.owner_changes().await else {
            return self.dispatch(EnginesInput::Listed(LlamaCpp::Unknown)).await;
        };
        loop {
            let found = self.link.look().await;
            self.dispatch(EnginesInput::Listed(found)).await;
            if owners.recv().await.is_none() {
                return;
            }
        }
    }
}

impl Machine for Engines {
    type Input = EnginesInput;
    type Effect = EnginesEffect;

    fn step(state: EnginesState, input: EnginesInput) -> (EnginesState, Vec<EnginesEffect>) {
        engines_step(state, input)
    }

    async fn run(&self, effect: EnginesEffect) -> Vec<EnginesInput> {
        match effect {
            EnginesEffect::RestartFailed => {
                self.live.notice(Notice::RestartFailed);
                Vec::new()
            }
            EnginesEffect::Look => vec![EnginesInput::Listed(self.link.look().await)],
            EnginesEffect::Restart => vec![EnginesInput::Restarted(self.link.restart().await)],
        }
    }
}
