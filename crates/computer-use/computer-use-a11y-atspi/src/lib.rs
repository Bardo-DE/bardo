//! The AT-SPI backend: the session's accessibility bus through the `atspi` crate. Chromium needs
//! `--force-renderer-accessibility`; Bardo apps expose their tree through `accesskit_unix`.
//!
//! The walk ([`read_window`]) is written against [`AtspiTree`], so it runs over any source of
//! objects; [`AtspiSource`] is that walk over the bus.

mod bus;
mod map;
mod tree;
mod walk;

use bus::BusTree;
use computer_use_a11y_tree::{A11yError, A11ySource, A11yTarget};
use computer_use_a11y_tree::{A11ySnapshot, NodeBudget};
use std::time::Duration;
use tokio::sync::OnceCell;

pub use tree::{AtspiTree, Extents, RawNode};
pub use walk::read_window;

/// How long a snapshot may take, reaching the bus included, before the run goes on without a
/// tree (the run's own limit on the effect is 30 s).
const SNAPSHOT_LIMIT: Duration = Duration::from_secs(8);

/// The accessibility bus, connected on first use.
#[derive(Debug, Default)]
pub struct AtspiSource {
    connection: OnceCell<zbus::Connection>,
}

impl AtspiSource {
    /// A source that has not connected yet; the first snapshot connects.
    #[must_use]
    pub fn new() -> Self {
        Self {
            connection: OnceCell::new(),
        }
    }

    /// A source over a connection that is already open (a host that connects itself).
    #[must_use]
    pub fn over(connection: zbus::Connection) -> Self {
        Self {
            connection: OnceCell::new_with(Some(connection)),
        }
    }

    /// Whether a connection is held.
    pub fn is_connected(&self) -> bool {
        self.connection.get().is_some()
    }

    async fn connected(&self) -> Result<&zbus::Connection, A11yError> {
        self.connection
            .get_or_try_init(|| async {
                let bus = ::atspi::AccessibilityConnection::new().await.map_err(|e| {
                    eprintln!(
                        "computer_use_daemon: the accessibility bus could not be reached: {e}"
                    );
                    A11yError::NoBus
                })?;
                Ok(bus.connection().clone())
            })
            .await
    }
}

impl A11ySource for AtspiSource {
    async fn snapshot(
        &self,
        target: &A11yTarget,
        budget: NodeBudget,
    ) -> Result<A11ySnapshot, A11yError> {
        // Reaching the bus counts against the limit too: a bus that never answers must not hold
        // the step until the run's own effect limit.
        let read = async {
            let tree = BusTree::over(self.connected().await?.clone());
            read_window(&tree, target, budget).await
        };
        match tokio::time::timeout(SNAPSHOT_LIMIT, read).await {
            Ok(snapshot) => snapshot,
            Err(_) => Err(A11yError::Timeout),
        }
    }
}
