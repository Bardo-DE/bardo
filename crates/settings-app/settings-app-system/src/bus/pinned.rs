//! A service confirmed to be the one the desktop trusts, and called by the owner that was
//! confirmed.

use super::activate::start_one;
use super::io::{because, confirmed, connect};
use account_client::AccountClient;
use account_client_dbus::Bus;
use inference_client::DbusTransport;
use settings_app_link::bus::BusSource;
use settings_app_model::Absence;
use settings_dbus::SenderName;
use zbus::Connection;
use zbus::names::UniqueName;

/// A connection, and the unique name of the owner that held a service's role when it was looked
/// for.
///
/// The role is the one in the desktop's caller table. A call goes to `owner`, never to the well-known name: a
/// name that changes hands between the check and the call is then not called, and the call fails
/// instead of reaching whoever took it.
#[derive(Debug, Clone)]
pub struct Pinned {
    pub connection: Connection,
    pub owner: SenderName,
}

impl Pinned {
    /// The owner as a destination for a proxy builder.
    #[must_use]
    pub fn destination(&self) -> String {
        self.owner.0.clone()
    }

    /// The owner as the unique name the accounts service's client sends to. A name the bus would not have given
    /// is a service that cannot be confirmed.
    fn unique(&self) -> Result<UniqueName<'static>, Absence> {
        UniqueName::try_from(self.owner.0.clone()).map_err(because(Absence::Unavailable))
    }

    /// The accounts service's bus transport with every call to accountd sent to the owner that was confirmed:
    /// the Spaces registry is reached from it (`bus().spaces()`).
    ///
    /// # Errors
    ///
    /// `Absence::Unavailable` when the owner's name is not a unique bus name.
    pub fn bus(&self) -> Result<Bus, Absence> {
        Ok(Bus::over(self.connection.clone()).accounts_at(self.unique()?))
    }

    /// The accounts service's client with every call to accountd (the sheets) sent to the owner that was
    /// confirmed.
    ///
    /// # Errors
    ///
    /// As [`Self::bus`].
    pub fn accounts(&self) -> Result<AccountClient<Bus>, Absence> {
        Ok(AccountClient::over(self.bus()?))
    }

    /// inferd's carrier with every call to inferd (the computers, the engines) sent to the owner
    /// that was confirmed.
    ///
    /// # Errors
    ///
    /// As [`Self::bus`].
    pub fn inference(&self) -> Result<DbusTransport, Absence> {
        Ok(DbusTransport::over(self.connection.clone()).inference_at(self.unique()?))
    }
}

/// The connection and the owner of the service `name`, only when it is already on the bus and
/// its owner holds the service's role. Nothing is started.
///
/// # Errors
///
/// `Absence::NotRunning` when nobody owns the name, `Absence::Unavailable` when its owner is not
/// the service, otherwise the failure of the bus.
pub async fn pinned(bus: &BusSource, name: &str) -> Result<Pinned, Absence> {
    let connection = connect(bus).await?;
    let owner = confirmed(bus, &connection, name).await?;
    Ok(Pinned { connection, owner })
}

/// As [`pinned`], but a service nobody owns yet is asked to start first.
///
/// That is for a call the person just asked for (adding an account, removing a Space) or a page
/// that is already starting it. The call then goes to whoever owns the name once it is up and holds the service's role, never
/// to the name itself.
///
/// # Errors
///
/// As [`pinned`], after the start.
pub async fn pinned_starting(bus: &BusSource, name: &str) -> Result<Pinned, Absence> {
    let connection = connect(bus).await?;
    let owner = match confirmed(bus, &connection, name).await {
        Err(Absence::NotRunning) => {
            start_one(&connection, name).await;
            confirmed(bus, &connection, name).await?
        }
        other => other?,
    };
    Ok(Pinned { connection, owner })
}

/// inferd and its confirmed owner, only when it is already on the bus: asking would start it, and
/// opening Settings must not. Every call goes to that owner.
///
/// # Errors
///
/// As [`pinned`].
pub async fn inferd(bus: &BusSource) -> Result<Pinned, Absence> {
    pinned(bus, account_wire::names::INFERENCE_BUS).await
}
