//! The places' one transition: `(state, input) -> (state, effects)`. A cloud account is turned on
//! only after the person confirmed; nothing here widens the setting on its own.

use action_settings::{CloudAllowed, Known, ModelChoice, PlaceName, Places, confirm_turn_on};

/// Which place a model is chosen for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaceRef {
    /// This computer.
    ThisComputer,
    /// One of the person's computers.
    Own(PlaceName),
    /// A cloud account.
    Cloud(PlaceName),
}

/// Where the places stand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacesState {
    /// The setting, with a row for every place the machine knows.
    pub places: Places,
    /// The cloud account being asked about, if any.
    pub confirming: Option<PlaceName>,
}

impl PlacesState {
    /// The state over `places`, nothing asked.
    #[must_use]
    pub fn of(places: Places) -> PlacesState {
        PlacesState {
            places,
            confirming: None,
        }
    }
}

/// What happened to the places.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlacesInput {
    /// The setting was read, and these places are known now.
    Read(Places, Known),
    /// The person flipped the switch of one of their computers.
    OwnSwitched {
        /// The computer.
        name: PlaceName,
        /// What it is now.
        to: bool,
    },
    /// The person flipped the switch of a cloud account.
    CloudSwitched {
        /// The account.
        name: PlaceName,
        /// What the person asked for.
        to: bool,
    },
    /// The person flipped "Ask me each time" of a cloud account that is on.
    AskEachTime {
        /// The account.
        name: PlaceName,
        /// Asking before each use, or not.
        to: bool,
    },
    /// The person said yes to the question before a cloud account is turned on.
    Confirm,
    /// The person said no to it: the account stays off.
    Cancel,
    /// The person picked a model for a place; none lets the assistant choose.
    Model {
        /// The place.
        place: PlaceRef,
        /// The model.
        model: Option<ModelChoice>,
    },
}

/// What the caller must do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlacesEffect {
    /// Write these places to the settings file.
    Write(Places),
}

/// The state after `input`, and what to do about it.
#[must_use]
pub fn places_step(state: PlacesState, input: PlacesInput) -> (PlacesState, Vec<PlacesEffect>) {
    let PlacesState {
        mut places,
        confirming,
    } = state;
    match input {
        PlacesInput::Read(read, known) => {
            let places = read.merged(&known);
            let confirming = confirming.filter(|name| places.cloud.iter().any(|c| &c.name == name));
            (PlacesState { places, confirming }, vec![])
        }
        PlacesInput::OwnSwitched { name, to } => {
            places
                .own
                .iter_mut()
                .filter(|own| own.name == name)
                .for_each(|own| own.allowed = to);
            written(places, confirming)
        }
        PlacesInput::CloudSwitched { name, to: true } => (
            PlacesState {
                places,
                confirming: Some(name),
            },
            vec![],
        ),
        PlacesInput::CloudSwitched { name, to: false } => {
            set_cloud(&mut places, &name, CloudAllowed::Off);
            written(places, None)
        }
        PlacesInput::AskEachTime { name, to } => {
            let allowed = if to {
                CloudAllowed::AskEachTime
            } else {
                CloudAllowed::On
            };
            let on = places
                .cloud
                .iter()
                .any(|c| c.name == name && c.allowed != CloudAllowed::Off);
            if on {
                set_cloud(&mut places, &name, allowed);
                written(places, confirming)
            } else {
                (PlacesState { places, confirming }, vec![])
            }
        }
        PlacesInput::Confirm => match confirming {
            Some(name) => {
                set_cloud(&mut places, &name, CloudAllowed::On);
                written(places, None)
            }
            None => (PlacesState::of(places), vec![]),
        },
        PlacesInput::Cancel => (PlacesState::of(places), vec![]),
        PlacesInput::Model { place, model } => {
            match place {
                PlaceRef::ThisComputer => places.this_computer = model,
                PlaceRef::Own(name) => places
                    .own
                    .iter_mut()
                    .filter(|own| own.name == name)
                    .for_each(|own| own.model.clone_from(&model)),
                PlaceRef::Cloud(name) => places
                    .cloud
                    .iter_mut()
                    .filter(|cloud| cloud.name == name)
                    .for_each(|cloud| cloud.model.clone_from(&model)),
            }
            written(places, confirming)
        }
    }
}

fn set_cloud(places: &mut Places, name: &PlaceName, allowed: CloudAllowed) {
    places
        .cloud
        .iter_mut()
        .filter(|cloud| &cloud.name == name)
        .for_each(|cloud| cloud.allowed = allowed);
}

fn written(places: Places, confirming: Option<PlaceName>) -> (PlacesState, Vec<PlacesEffect>) {
    let effect = PlacesEffect::Write(places.clone());
    (PlacesState { places, confirming }, vec![effect])
}

impl PlacesState {
    /// The question before a cloud account is turned on, as a title and the line under it: the
    /// words are the actions service's, split where the question ends.
    #[must_use]
    pub fn question(&self) -> Option<(String, String)> {
        let name = self.confirming.as_ref()?;
        let provider = self
            .places
            .cloud
            .iter()
            .find(|cloud| &cloud.name == name)
            .map(|cloud| cloud.provider.as_str())?;
        let whole = confirm_turn_on(name, provider);
        let (title, message) = whole.split_once("? ")?;
        Some((format!("{title}?"), message.to_owned()))
    }
}
