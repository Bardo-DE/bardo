//! What the fuzz runner needs from a compositor: the fixture, or a stand-in in the runner's own
//! tests.

use crate::fixture::{ClientId, Fixture};
use compositor_core::InvariantBreak;
use compositor_core::policy::ConnectionRole;
use compositor_testclient::TestClient;
use std::time::Duration;

/// A compositor the runner can drive.
pub trait FuzzHost {
    /// A new client of `role`, already pumped.
    fn connect(&mut self, role: ConnectionRole) -> ClientId;
    /// A client.
    fn client(&mut self, id: ClientId) -> &mut TestClient;
    /// Pumps every party until quiet.
    fn pump(&mut self) -> bool;
    /// Moves the clock.
    fn advance(&mut self, by: Duration);
    /// The model's invariants (oracle 4).
    ///
    /// # Errors
    ///
    /// [`InvariantBreak`] naming the first invariant the compositor no longer holds.
    fn invariants(&self) -> Result<(), InvariantBreak>;
}

impl FuzzHost for Fixture {
    fn connect(&mut self, role: ConnectionRole) -> ClientId {
        Fixture::connect(self, role)
    }

    fn client(&mut self, id: ClientId) -> &mut TestClient {
        Fixture::client(self, id)
    }

    fn pump(&mut self) -> bool {
        Fixture::pump(self)
    }

    fn advance(&mut self, by: Duration) {
        Fixture::advance(self, by);
    }

    fn invariants(&self) -> Result<(), InvariantBreak> {
        self.model().check_invariants()
    }
}
