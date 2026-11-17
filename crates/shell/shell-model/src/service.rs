//! What every service is, from the outside: the state it publishes, the commands it takes, and the
//! two halves of the channels that join them.
//!
//! A service is a marker type implementing [`Service`], beside its state in the module that names
//! it (`shell_model::clock::ClockService`). The task that does its work, and the effects it
//! needs, are `shell-services`' `Run`; a surface needs only the [`Handle`] a running service gives:
//! a watch receiver of its state and a [`Control`] for its commands.

use std::fmt;

use tokio::sync::{mpsc, watch};

/// One service: what it publishes and what it can be told. No I/O.
pub trait Service: Sized + 'static {
    /// The name its log lines carry.
    const NAME: &'static str;
    /// What it publishes; its default means "absent" or "nothing yet".
    type State: Clone + Default + PartialEq + Send + Sync + 'static;
    /// What it can be told.
    type Command: Send + 'static;
}

/// The service has stopped: its commands go nowhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[error("the service has stopped")]
pub struct ServiceGoneError;

/// Sends commands of type `C` to one running service. Cheap to clone; every surface holds one
/// through its `Services`.
#[derive(Debug)]
pub struct Control<C> {
    tx: mpsc::UnboundedSender<C>,
}

impl<C> Clone for Control<C> {
    fn clone(&self) -> Self {
        Control {
            tx: self.tx.clone(),
        }
    }
}

impl<C> Control<C> {
    /// A control and the receiver its service task drains.
    #[must_use]
    pub fn channel() -> (Control<C>, mpsc::UnboundedReceiver<C>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Control { tx }, rx)
    }

    /// Queue `command`.
    ///
    /// # Errors
    /// [`Gone`] when the service's receiver has been dropped.
    #[expect(
        clippy::map_err_ignore,
        reason = "the send error only hands the command back; `Gone` says all there is to say"
    )]
    pub fn send(&self, command: C) -> Result<(), ServiceGoneError> {
        self.tx.send(command).map_err(|_| ServiceGoneError)
    }

    /// Resolves once the service's receiver has been dropped: the service has stopped.
    pub async fn closed(&self) {
        self.tx.closed().await;
    }
}

/// The service's own halves of its channels.
pub struct ServiceIo<S: Service> {
    /// Where it publishes.
    pub state: watch::Sender<S::State>,
    /// What it is told; `None` once every [`Control`] has gone.
    pub commands: mpsc::UnboundedReceiver<S::Command>,
}

/// A running service as its readers hold it. Cheap to clone.
pub struct Handle<S: Service> {
    pub state: watch::Receiver<S::State>,
    pub control: Control<S::Command>,
}

impl<S: Service> fmt::Debug for ServiceIo<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceIo")
            .field("service", &S::NAME)
            .finish()
    }
}

impl<S: Service> Clone for Handle<S> {
    fn clone(&self) -> Self {
        Handle {
            state: self.state.clone(),
            control: self.control.clone(),
        }
    }
}

impl<S: Service> fmt::Debug for Handle<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handle").field("service", &S::NAME).finish()
    }
}

impl<S: Service> Handle<S> {
    /// A handle starting at `state`, and the service's halves of its channels: what a service's
    /// task is given, and what a test or a demo feeds and reads in its place.
    #[must_use]
    pub fn channel(state: S::State) -> (Handle<S>, ServiceIo<S>) {
        let (tx, rx) = watch::channel(state);
        let (control, commands) = Control::channel();
        (
            Handle { state: rx, control },
            ServiceIo {
                state: tx,
                commands,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Control, ServiceGoneError};

    #[test]
    fn a_command_reaches_the_service_until_it_stops() {
        let (control, mut rx) = Control::<u8>::channel();
        let clone = control.clone();
        control.send(1).unwrap();
        clone.send(2).unwrap();
        assert_eq!(rx.try_recv().ok(), Some(1));
        assert_eq!(rx.try_recv().ok(), Some(2));
        drop(rx);
        assert_eq!(control.send(3), Err(ServiceGoneError));
    }
}
