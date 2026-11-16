//! What a method answers instead of a value: a refusal as the error
//! `org.bardo.Accounts1.Error.<Refusal>` (`account_dbus::accounts::refusal_error_name` pins the names), or
//! one of the bus's own (`AccessDenied`, `InvalidArgs`, `UnknownObject`) under its real name, so
//! a client classifies it as it does a bus's refusal.
//!
//! Written by hand rather than with the `DBusError` derive: the derive names every non-refusal
//! `org.freedesktop.zbus.Error`, which a client cannot tell from any other failure.

use account_dbus::accounts::{ServiceRoot, launcher_fault_error_name_in, refusal_error_name};
use account_wire::LauncherFault;
use account_wire::Refusal;
use zbus::DBusError;
use zbus::fdo;
use zbus::message::{Header, Message};
use zbus::names::ErrorName;

/// An error reply: its name and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusedError {
    name: ErrorName<'static>,
    text: String,
}

impl RefusedError {
    /// The error for `refusal`.
    #[must_use]
    pub fn of(refusal: Refusal) -> Self {
        let name = ErrorName::try_from(refusal_error_name(refusal)).unwrap_or_else(|_| {
            ErrorName::from_static_str_unchecked("org.freedesktop.DBus.Error.Failed")
        });
        Self {
            name,
            text: String::new(),
        }
    }

    /// The bus's own error `error`, under its name.
    fn bus(error: &fdo::Error) -> Self {
        Self {
            name: error.name().into_owned(),
            text: error.description().unwrap_or_default().to_owned(),
        }
    }

    /// Something the bus refused while a Request object was being made (account-sheets), under
    /// the bus's own name.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "used as a `map_err` function, which hands the error over by value"
    )]
    pub(crate) fn from_request(error: fdo::Error) -> Self {
        Self::bus(&error)
    }

    /// A launcher's call accountd refuses, under the fault's own typed name.
    pub(crate) fn launcher(fault: LauncherFault, why: impl Into<String>) -> Self {
        Self {
            name: ErrorName::try_from(launcher_fault_error_name_in(fault, ServiceRoot::Accounts))
                .unwrap_or_else(|_| {
                    ErrorName::from_static_str_unchecked("org.freedesktop.DBus.Error.Failed")
                }),
            text: why.into(),
        }
    }

    /// An argument that is not what the interface says.
    pub(crate) fn invalid(why: impl Into<String>) -> Self {
        Self::bus(&fdo::Error::InvalidArgs(why.to_string()))
    }

    /// A caller the daemon will not answer.
    pub(crate) fn access_denied(why: impl Into<String>) -> Self {
        Self::bus(&fdo::Error::AccessDenied(why.to_string()))
    }

    /// An object that does not exist for this caller.
    pub(crate) fn unknown_object(why: impl Into<String>) -> Self {
        Self::bus(&fdo::Error::UnknownObject(why.to_string()))
    }

    /// A caller that already has what it asks for under way (a sheet of that kind open):
    /// `org.freedesktop.DBus.Error.LimitsExceeded`.
    pub(crate) fn busy(why: impl Into<String>) -> Self {
        Self::bus(&fdo::Error::LimitsExceeded(why.to_string()))
    }

    /// Something the daemon could not do.
    pub(crate) fn failed(why: impl Into<String>) -> Self {
        Self::bus(&fdo::Error::Failed(why.to_string()))
    }

    /// The error's D-Bus name.
    #[must_use]
    pub fn error_name(&self) -> &str {
        self.name.as_str()
    }
}

impl DBusError for RefusedError {
    fn create_reply(&self, msg: &Header<'_>) -> zbus::Result<Message> {
        Message::error(msg, self.name.clone())?.build(&(self.text.as_str(),))
    }

    fn name(&self) -> ErrorName<'_> {
        self.name.clone()
    }

    fn description(&self) -> Option<&str> {
        Some(self.text.as_str())
    }
}
