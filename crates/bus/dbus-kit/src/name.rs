//! Taking a well-known name.

use zbus::Connection;
use zbus::fdo::{RequestNameFlags, RequestNameReply};

/// Why a name was not taken.
#[derive(Debug, thiserror::Error)]
pub enum ClaimError {
    /// Another connection owns the name, or waits for it.
    #[error("{0} is taken")]
    Taken(String),
    /// The bus refused the request.
    #[error("the bus refused the name {name}: {source}")]
    Bus {
        /// The name asked for.
        name: String,
        /// What the bus said.
        #[source]
        source: zbus::Error,
    },
}

/// Asks for `name` on `connection` and never queues for it: a second owner of a name the first
/// serves (two routers, two journals) would be a fault, not a wait.
///
/// # Errors
///
/// [`ClaimError::Taken`] when the name has an owner; [`ClaimError::Bus`] when the bus refuses.
pub async fn claim(connection: &Connection, name: &str) -> Result<(), ClaimError> {
    match connection
        .request_name_with_flags(name, RequestNameFlags::DoNotQueue.into())
        .await
    {
        Ok(RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner) => Ok(()),
        Ok(RequestNameReply::InQueue | RequestNameReply::Exists) | Err(zbus::Error::NameTaken) => {
            Err(ClaimError::Taken(name.to_owned()))
        }
        Err(source) => Err(ClaimError::Bus {
            name: name.to_owned(),
            source,
        }),
    }
}

impl From<ClaimError> for zbus::Error {
    /// A name that is taken is zbus's own [`zbus::Error::NameTaken`]; a refusal is what the bus
    /// said.
    fn from(error: ClaimError) -> Self {
        match error {
            ClaimError::Taken(_) => zbus::Error::NameTaken,
            ClaimError::Bus { source, .. } => source,
        }
    }
}
