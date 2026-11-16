//! Every closed vocabulary of the companion's components parses back from its own slug, and no
//! two members share one.

use ui_components::components::companion::answer::action::ActionRole;
use ui_components::components::companion::answer::footer::ServedPlaceKind;
use ui_components::components::companion::memory::model::{FactOrigin, FactStanding};
use ui_components::components::companion::orb::model::{OrbPeriod, OrbSize, OrbTone};
use ui_components::components::companion::plan::model::{PlanFinish, StopWhy};
use ui_components::components::companion::prompt::host::PromptHost;
use ui_components::components::companion::prompt::model::PromptPhase;
use ui_components::components::companion::proposed_event::model::BusyKind;
use ui_components::components::companion::run_row::model::{RunPlace, RunState};
use ui_core::word::Word;

fn round_trips<T: Word + std::fmt::Debug>() {
    for &word in T::ALL {
        assert_eq!(T::parse(word.slug()), Some(word), "{word:?}");
    }
    let slugs: std::collections::HashSet<_> = T::ALL.iter().map(|word| word.slug()).collect();
    assert_eq!(slugs.len(), T::ALL.len(), "{:?}", T::ALL);
}

#[test]
fn every_companion_word_parses_back_from_its_slug() {
    round_trips::<ActionRole>();
    round_trips::<BusyKind>();
    round_trips::<FactOrigin>();
    round_trips::<FactStanding>();
    round_trips::<OrbPeriod>();
    round_trips::<OrbSize>();
    round_trips::<OrbTone>();
    round_trips::<PlanFinish>();
    round_trips::<PromptHost>();
    round_trips::<PromptPhase>();
    round_trips::<RunPlace>();
    round_trips::<RunState>();
    round_trips::<ServedPlaceKind>();
    round_trips::<StopWhy>();
}
