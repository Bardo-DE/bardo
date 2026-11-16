//! The policy writer over inferd. The writer itself is `agent-models`' `TransportWriter` (it
//! reads the person's own turns and the action catalogue, never content; nothing in a draft is
//! believed as written); this adds the constructor over the session bus.

use action_core::{ActionCard, Derived, PolicyWriter, UserTurn};
use action_dbus::InferLink;
use agent_models::TransportWriter;
use inference_client::Transport;
use permission_core::ReviewError;
use provenance::{SpaceId, TaskId};

/// The policy writer: reads the person's turns and the action catalogue, never content.
#[derive(Debug)]
pub struct InferdWriter<T: Transport>(TransportWriter<T>);

impl<T: Transport> InferdWriter<T> {
    /// Asks through `transport`.
    pub fn new(transport: T) -> Self {
        Self(TransportWriter::new(transport))
    }
}

impl InferdWriter<InferLink> {
    /// Asks inferd over the session bus.
    #[must_use]
    pub fn on_bus(connection: &action_dbus::BusConnection, tap: action_dbus::InferWatch) -> Self {
        Self::new(crate::infer::inferd_transport(connection, tap))
    }
}

impl<T: Transport> PolicyWriter for InferdWriter<T> {
    async fn derive(
        &self,
        task: &TaskId,
        turns: &[UserTurn],
        catalogue: &[ActionCard],
        space: &SpaceId,
    ) -> Result<Derived, ReviewError> {
        self.0.derive(task, turns, catalogue, space).await
    }
}
