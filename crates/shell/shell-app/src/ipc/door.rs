//! The daemon's single-instance door, shared by the thread that accepts on it and the task that
//! asks whether the lock file is still the one held ([`crate::door_guard`]).
//!
//! A `service_locator::Listening` unlinks its socket path when it is dropped. After the lock file has
//! been removed (a cache cleaner ran) a second daemon may have bound that same path, and the
//! first one's drop would then delete the second one's socket. So a [`Door`] that is dropped
//! while [displaced](Door::displaced) keeps the `Listening` alive for good: the process is ending, and
//! the descriptors go with it.

use std::io;
use std::sync::Arc;

use service_locator::{Listening, Stream};

/// A shared handle on the daemon's `Listening`. Cloning shares it.
#[derive(Debug, Clone)]
pub struct Door(Arc<Listening>);

impl From<Listening> for Door {
    fn from(listening: Listening) -> Self {
        Door(Arc::new(listening))
    }
}

impl Door {
    /// Wait for the next client.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of accepting the connection.
    pub fn accept(&self) -> io::Result<Stream> {
        self.0.accept()
    }

    /// Is the lock file no longer the one this daemon holds?
    #[must_use]
    pub fn displaced(&self) -> bool {
        self.0.displaced()
    }
}

impl Drop for Door {
    fn drop(&mut self) {
        if self.displaced() {
            // One count that is never given back: the `Listening` is never dropped, so its drop
            // never unlinks a socket path that now belongs to another daemon.
            std::mem::forget(Arc::clone(&self.0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Door;
    use base_testkit::Scratch;

    fn door_in(dir: &Scratch) -> (service_locator::Agent, Door) {
        let env = service_locator::Environment {
            runtime_dir: Some(dir.path().as_os_str().to_owned()),
            ..Default::default()
        };
        let agent = service_locator::Agent::in_environment("door", service_locator::here(), &env)
            .unwrap_or_else(|e| panic!("building the agent: {e}"));
        let listening = agent.listen().unwrap_or_else(|e| panic!("listening: {e}"));
        (agent, Door::from(listening))
    }

    #[test]
    fn a_door_dropped_after_its_lock_file_went_leaves_the_socket_alone() {
        let dir = Scratch::new("door-displaced");
        let (agent, door) = door_in(&dir);
        let socket = agent
            .socket()
            .unwrap_or_else(|| panic!("a socket address"))
            .to_path_buf();
        assert!(
            socket.exists(),
            "the socket is there while the door is held"
        );
        std::fs::remove_file(&agent.address().lock).unwrap_or_else(|e| panic!("{e}"));
        assert!(door.displaced());
        // Stands for the second daemon having bound the same path: the first must not unlink it.
        drop(door);
        assert!(
            socket.exists(),
            "a displaced door's drop unlinked the socket"
        );
    }

    #[test]
    fn a_door_dropped_while_ours_unlinks_its_socket() {
        let dir = Scratch::new("door-ours");
        let (agent, door) = door_in(&dir);
        let socket = agent
            .socket()
            .unwrap_or_else(|| panic!("a socket address"))
            .to_path_buf();
        drop(door);
        assert!(!socket.exists(), "the socket was left behind");
    }
}
