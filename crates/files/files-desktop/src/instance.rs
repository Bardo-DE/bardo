//! One running app: the first launch owns `org.bardo.Files`, a later one hands its request to it.
//!
//! The first launch also offers `org.freedesktop.FileManager1`, unless another file manager already
//! owns that name: the name is asked for without queueing and without replacing, so a person's
//! chosen file manager is never taken over. [`Served::owns_file_manager`] says which it was, so the
//! window can offer to become the default.

pub use zbus::Connection as Bus;
use zbus::Connection;
use zbus::fdo::{DBusProxy, RequestNameFlags, RequestNameReply};
use zbus::names::BusName;

pub use crate::error::InstanceError;
use crate::file_manager::{self, Show, Sink};

/// The name the app owns while it runs.
pub const APP_NAME: &str = "org.bardo.Files";

/// The running app: holds the connection, and with it the names.
#[derive(Debug, Clone)]
pub struct Served {
    owns_file_manager: bool,
    _connection: Connection,
}

impl Served {
    /// Whether the file manager name is ours: programs that ask the desktop for "the file
    /// manager" reach this app. False when another file manager has it; this app does not take it.
    #[must_use]
    pub fn owns_file_manager(&self) -> bool {
        self.owns_file_manager
    }
}

/// The answer to [`claim`].
#[derive(Debug, Clone)]
pub enum Claim {
    /// This launch is the app. Keep the [`Served`] for as long as it runs.
    First(Served),
    /// Another one is running; [`forward`] this launch's request to it and exit.
    Running,
}

/// A connection to the person's session bus, driven by the runtime the caller is in.
pub async fn session() -> Result<Connection, InstanceError> {
    Ok(Connection::session().await?)
}

/// Becomes the running app if none is, serving each request that arrives at `sink`.
pub async fn claim(connection: &Connection, sink: Sink) -> Result<Claim, InstanceError> {
    // The object goes up before the names, so a call right after the name appears is answered.
    file_manager::serve(connection, sink).await?;
    zbus_ready::serving(connection).await?;
    if !ask_for(connection, APP_NAME).await? {
        file_manager::unserve(connection).await;
        return Ok(Claim::Running);
    }
    let owns_file_manager = ask_for(connection, file_manager::INTERFACE).await?;
    Ok(Claim::First(Served {
        owns_file_manager,
        _connection: connection.clone(),
    }))
}

/// Asks the running app to show `show`.
pub async fn forward(connection: &Connection, show: &Show) -> Result<(), InstanceError> {
    let name = BusName::try_from(APP_NAME).map_err(zbus::Error::from)?;
    let owner = match DBusProxy::new(connection).await?.get_name_owner(name).await {
        Ok(owner) => owner,
        Err(zbus::fdo::Error::NameHasNoOwner(_)) => return Err(InstanceError::NotRunning),
        Err(error) => return Err(zbus::Error::from(error).into()),
    };
    let call = show.to_call();
    connection
        .call_method(
            Some(owner.as_str()),
            file_manager::OBJECT_PATH,
            Some(file_manager::INTERFACE),
            call.method,
            &(call.uris, call.startup_id),
        )
        .await?;
    Ok(())
}

/// Asks for `name` without queueing and without replacing its owner: whether it is ours now.
async fn ask_for(connection: &Connection, name: &'static str) -> Result<bool, zbus::Error> {
    match connection
        .request_name_with_flags(name, RequestNameFlags::DoNotQueue.into())
        .await
    {
        Ok(RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner) => Ok(true),
        Ok(RequestNameReply::Exists | RequestNameReply::InQueue) => Ok(false),
        Err(zbus::Error::NameTaken) => Ok(false),
        Err(error) => Err(error),
    }
}
