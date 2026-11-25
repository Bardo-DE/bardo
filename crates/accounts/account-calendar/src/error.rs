//! Why a calendar store did not do what it was asked.

use http_core::{Cause, HttpError};
use std::error::Error;
use std::sync::Arc;

/// Why a calendar store did not do what it was asked.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CalendarError {
    /// No answer: the seam could not send the request.
    #[error("the calendar store could not be reached: {0}")]
    Http(#[from] HttpError),
    /// `401` or `403`: the sign-in, or the access it carries, was refused.
    #[error("the calendar store refused the sign-in")]
    Unauthorized,
    /// `412`: the event changed in the store since it was read, so it was not overwritten.
    #[error("the event changed in the calendar store since it was read")]
    Changed,
    /// Any other status the exchange could not go on from.
    #[error("the calendar store answered {status} to {what}")]
    Status {
        /// The HTTP status.
        status: u16,
        /// What was asked.
        what: &'static str,
    },
    /// The answer was not the document asked for.
    #[error("the calendar store's answer could not be read: {what}")]
    Malformed {
        /// What could not be read.
        what: String,
        /// Why, when a reader said.
        #[source]
        source: Option<Cause>,
    },
    /// The platform's calendar service refused or failed a call (Evolution Data Server over the
    /// bus, or an app's own store).
    #[error("the calendar service failed: {what}")]
    Service {
        /// What was asked.
        what: String,
        /// Why, when the service said.
        #[source]
        source: Option<Cause>,
    },
}

impl CalendarError {
    /// An answer that was not the document asked for, with nothing more to say why.
    pub fn malformed(what: impl Into<String>) -> Self {
        CalendarError::Malformed {
            what: what.into(),
            source: None,
        }
    }

    /// An answer `source` could not read.
    pub fn unreadable(what: impl Into<String>, source: impl Error + Send + Sync + 'static) -> Self {
        CalendarError::Malformed {
            what: what.into(),
            source: Some(Arc::new(source)),
        }
    }

    /// A call the calendar service failed, because of `source`.
    pub fn service(what: impl Into<String>, source: impl Error + Send + Sync + 'static) -> Self {
        CalendarError::Service {
            what: what.into(),
            source: Some(Arc::new(source)),
        }
    }

    /// A call the calendar service failed, with nothing more to say why.
    pub fn service_failed(what: impl Into<String>) -> Self {
        CalendarError::Service {
            what: what.into(),
            source: None,
        }
    }

    /// The error a status that is not a success stands for.
    pub(crate) fn of_status(status: u16, what: &'static str) -> Self {
        match status {
            401 | 403 => CalendarError::Unauthorized,
            412 => CalendarError::Changed,
            status => CalendarError::Status { status, what },
        }
    }
}
