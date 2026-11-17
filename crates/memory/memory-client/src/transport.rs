//! How a request reaches memoryd: in process, nowhere (other desktops), or over D-Bus.

use memory_core::{MemoryReply, MemoryRequest};

/// Why a request did not get an answer.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransportError {
    /// There is no memory on this desktop: writers treat it as a no-op.
    #[error("no memory on this desktop")]
    Absent,
    /// The connection closed.
    #[error("connection closed")]
    Closed,
    /// The bus failed.
    #[error("bus: {0}")]
    Bus(String),
}

/// A way to send one request and get one reply. The caller identity is the transport's: the
/// bus derives it from the connection, in-process it is configured.
pub trait Transport: Send + Sync {
    /// Sends `request`.
    fn call(
        &self,
        request: MemoryRequest,
    ) -> impl Future<Output = Result<MemoryReply, TransportError>> + Send;
}

#[cfg(feature = "in_process")]
pub use in_process::InProcess;

#[cfg(feature = "in_process")]
mod in_process {
    use super::{Transport, TransportError};
    use memory_core::{MemoryReply, MemoryRequest};
    use memory_service::{Backend, Caller, MemoryService};
    use std::sync::Arc;

    /// The app hosts the service itself (tests, single-process embedders). Feature `in_process`.
    ///
    /// ```
    /// use memory_client::{InProcess, Memory, Recorded};
    /// use memory_core::{AppId, Isolation};
    /// use memory_fake::{ScriptedConsolidator, fake_service, mail, mail_thread_archived};
    /// use memory_service::Caller;
    /// use std::sync::Arc;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # tokio::runtime::Builder::new_current_thread().build()?.block_on(async {
    /// let service = Arc::new(fake_service(ScriptedConsolidator::default()));
    /// let caller = Caller::app(AppId { name: mail(), isolation: Isolation::InProcess });
    /// let app = Memory::over(InProcess::new(service, caller));
    /// let record = mail_thread_archived().ok_or("fixture")?;
    /// assert!(matches!(app.record(record).await?, Recorded::Stored(_)));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # })
    /// # }
    /// ```
    pub struct InProcess<B: Backend> {
        service: Arc<MemoryService<B>>,
        caller: Caller,
    }

    impl<B: Backend> std::fmt::Debug for InProcess<B> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("InProcess")
                .field("caller", &self.caller)
                .finish_non_exhaustive()
        }
    }

    impl<B: Backend> InProcess<B> {
        /// Calls `service` as `caller`.
        pub fn new(service: Arc<MemoryService<B>>, caller: Caller) -> Self {
            Self { service, caller }
        }
    }

    impl<B: Backend> Transport for InProcess<B> {
        async fn call(&self, request: MemoryRequest) -> Result<MemoryReply, TransportError> {
            Ok(self.service.handle(&self.caller, request).await)
        }
    }
}

/// No memory here (another desktop, or memoryd is not installed): every call is `Absent`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Absent;

impl Transport for Absent {
    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn call(&self, _request: MemoryRequest) -> Result<MemoryReply, TransportError> {
        Err(TransportError::Absent)
    }
}

/// `org.bardo.Memory1` on the session bus.
#[cfg(feature = "dbus")]
#[derive(Debug)]
pub struct DbusTransport {
    connection: memory_dbus::BusConnection,
}

#[cfg(feature = "dbus")]
impl DbusTransport {
    /// Over an existing session-bus connection.
    #[must_use]
    pub fn new(connection: memory_dbus::BusConnection) -> Self {
        Self { connection }
    }

    /// The connection.
    pub fn connection(&self) -> &memory_dbus::BusConnection {
        &self.connection
    }
}

#[cfg(feature = "dbus")]
impl DbusTransport {
    /// Sends `call` and reads its reply; a refusal is `MemoryReply::Refused`, a missing daemon
    /// is `Absent`.
    async fn send(
        &self,
        call: memory_dbus::Call,
        fd: Option<std::os::fd::OwnedFd>,
    ) -> Result<MemoryReply, TransportError> {
        use memory_dbus::Failure;
        match memory_dbus::invoke(&self.connection, &call, fd).await {
            Ok(outputs) => memory_dbus::decode_reply(&call, &outputs)
                .map_err(|e| TransportError::Bus(e.to_string())),
            Err(error) => match error.failure() {
                Failure::Refused(refusal) => Ok(MemoryReply::Refused(refusal)),
                Failure::NoDaemon => Err(TransportError::Absent),
                Failure::Closed => Err(TransportError::Closed),
                Failure::Other(why) => Err(TransportError::Bus(why)),
            },
        }
    }

    /// Writes a tar export of `options` to `out` and answers its manifest. `Export` needs a
    /// stream, which `Transport::call` cannot carry, so it has this method of its own.
    ///
    /// # Errors
    /// [`TransportError`] when the request cannot be encoded or sent, or the connection closes.
    pub async fn export(
        &self,
        options: memory_core::ExportOptions,
        out: std::os::fd::OwnedFd,
    ) -> Result<MemoryReply, TransportError> {
        let call = memory_dbus::encode_request(&MemoryRequest::Export(options))
            .map_err(|e| TransportError::Bus(e.to_string()))?;
        self.send(call, Some(out)).await
    }
}

#[cfg(feature = "dbus")]
impl Transport for DbusTransport {
    async fn call(&self, request: MemoryRequest) -> Result<MemoryReply, TransportError> {
        if matches!(request, MemoryRequest::Export(_)) {
            return Err(TransportError::Bus(
                "Export needs a stream: use DbusTransport::export".into(),
            ));
        }
        let call = memory_dbus::encode_request(&request)
            .map_err(|e| TransportError::Bus(e.to_string()))?;
        self.send(call, None).await
    }
}
