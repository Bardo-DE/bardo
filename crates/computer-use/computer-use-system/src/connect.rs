//! cuad's links to the daemons it works with, over one session-bus connection: the router
//! (`Intents1`) for the gate and the message path, memoryd (`Memory1`) for the event log and
//! inferd (`Inference1`) for the model.

use crate::model::InferdCuaModel;
use memory_client::Memory;
use computer_use_host::DaemonLinks;
use computer_use_host::IntentsGate;
use action_client::Intents;
use account_capability::Tier;
use zbus::Connection;

/// The links of the running daemon, as `SystemSeams` names their types.
#[derive(Debug)]
pub struct Links {
    /// The router's gate.
    pub gate: IntentsGate<action_client::DbusTransport>,
    /// The event log and the message path.
    pub events: DaemonLinks<memory_client::DbusTransport, action_client::DbusTransport>,
    /// The model, over inferd on the same bus.
    pub model: InferdCuaModel<inference_client::DbusTransport>,
}

impl Links {
    /// Links that reach intentd, memoryd and inferd through `connection`. Nothing is called
    /// yet: a daemon that is not there is found when it is first asked, and each seam says so
    /// in its own words (`LinkError::Unavailable`, `Transient`).
    pub fn over(connection: &Connection, tier: Tier) -> Links {
        Links {
            gate: IntentsGate::over(action_client::DbusTransport::new(connection.clone())),
            events: DaemonLinks::over(
                Memory::over(memory_client::DbusTransport::new(connection.clone())),
                Intents::over(action_client::DbusTransport::new(connection.clone())),
            ),
            model: InferdCuaModel::over(inference_client::DbusTransport::over(connection.clone()))
                .on_tier(tier),
        }
    }
}
