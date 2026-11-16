//! The reviewer cascade over inferd, from the configured set or the placeholder one. The set,
//! its cards and the placeholder are `docket-models`'; this makes each stage's link on the bus.

use action_review::{InferReviewer, ModelChoice, ReviewerSet};
use action_core::ReviewTimeouts;
use action_dbus::InferWatch;
use action_dbus::{BusConnection, InferLink};

use crate::infer::InferdModel;

pub use agent_models::placeholder_set;

fn model(
    connection: &BusConnection,
    choice: &ModelChoice,
    tap: &InferWatch,
) -> InferdModel<InferLink> {
    InferdModel::on_bus(connection, agent_models::card_of(choice), tap.clone())
}

/// The cascade for `set` (the placeholder one when none is given), with `timeouts` per stage, its
/// links behind `tap`.
pub fn reviewer(
    connection: &BusConnection,
    set: Option<&ReviewerSet>,
    timeouts: ReviewTimeouts,
    tap: &InferWatch,
) -> Option<InferReviewer<InferdModel<InferLink>>> {
    let placeholder = placeholder_set();
    let set = set.or(placeholder.as_ref())?;
    Some(InferReviewer {
        quick: model(connection, &set.quick, tap),
        deliberate: model(connection, &set.deliberate, tap),
        second: model(connection, &set.second, tap),
        timeouts,
    })
}
