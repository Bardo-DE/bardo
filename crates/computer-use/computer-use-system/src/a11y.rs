//! The accessibility source of the running daemon. With no tree (`computer-use.toml` `a11y = "off"`) the
//! seam is `None`, and the computer-use daemon still starts, answers the bus, and says what is missing.

use computer_use_a11y_atspi::AtspiSource;
use computer_use_a11y_tree::{A11yError, A11ySnapshot, A11ySource, A11yTarget, NodeBudget};

/// AT-SPI, with one line logged per tree read.
#[derive(Debug)]
pub struct LoggedAtspi(pub AtspiSource);

impl A11ySource for LoggedAtspi {
    async fn snapshot(
        &self,
        target: &A11yTarget,
        budget: NodeBudget,
    ) -> Result<A11ySnapshot, A11yError> {
        let started = std::time::Instant::now();
        let snapshot = self.0.snapshot(target, budget).await;
        // One line per tree read: its size, or why a run goes on from pixels alone.
        let ms = started.elapsed().as_millis();
        match &snapshot {
            Ok(tree) => eprintln!(
                "computer_use_daemon: accessibility tree: {} nodes in {ms} ms",
                tree.nodes.len()
            ),
            Err(error) => {
                eprintln!("computer_use_daemon: no accessibility tree after {ms} ms: {error:?}")
            }
        }
        snapshot
    }
}
