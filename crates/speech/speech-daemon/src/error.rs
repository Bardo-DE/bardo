//! `org.bardo.Voice1.Error.<Variant>`: the bus form of a refused call. The six refusals are
//! `VoiceRefusedError`'s, one to one; `Malformed` is a body that does not parse and `Gone` is a daemon
//! that is shutting down.

use speech_wire::{ERROR_PREFIX, VoiceRefusedError};
use zbus::fdo;

/// The errors the members return.
#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "org.bardo.Voice1.Error")]
pub enum VoiceError {
    /// zbus's own failures.
    #[zbus(error)]
    ZBus(zbus::Error),
    /// Voice is off.
    Disabled(String),
    /// First-use consent is pending.
    NeedsConsent(String),
    /// Busy.
    Busy(String),
    /// This caller may not.
    NotAllowed(String),
    /// No speech model.
    NoModel(String),
    /// No microphone.
    MicUnavailable(String),
    /// The body does not parse.
    Malformed(String),
    /// The daemon is going away.
    Gone(String),
}

impl From<VoiceRefusedError> for VoiceError {
    /// Every refusal a call gets passes through here, so this is where it is logged.
    fn from(refusal: VoiceRefusedError) -> Self {
        tracing::info!(?refusal, "voice call refused");
        let why = refusal.to_string();
        match refusal {
            VoiceRefusedError::Disabled => VoiceError::Disabled(why),
            VoiceRefusedError::NeedsConsent => VoiceError::NeedsConsent(why),
            VoiceRefusedError::Busy => VoiceError::Busy(why),
            VoiceRefusedError::NotAllowed => VoiceError::NotAllowed(why),
            VoiceRefusedError::NoModel => VoiceError::NoModel(why),
            VoiceRefusedError::MicUnavailable => VoiceError::MicUnavailable(why),
        }
    }
}

impl From<fdo::Error> for VoiceError {
    fn from(error: fdo::Error) -> Self {
        VoiceError::ZBus(error.into())
    }
}

impl VoiceError {
    /// The refusal this error carries, or `None` for anything else.
    #[must_use]
    pub fn refusal(&self) -> Option<VoiceRefusedError> {
        match self {
            VoiceError::Disabled(_) => Some(VoiceRefusedError::Disabled),
            VoiceError::NeedsConsent(_) => Some(VoiceRefusedError::NeedsConsent),
            VoiceError::Busy(_) => Some(VoiceRefusedError::Busy),
            VoiceError::NotAllowed(_) => Some(VoiceRefusedError::NotAllowed),
            VoiceError::NoModel(_) => Some(VoiceRefusedError::NoModel),
            VoiceError::MicUnavailable(_) => Some(VoiceRefusedError::MicUnavailable),
            VoiceError::ZBus(_) | VoiceError::Malformed(_) | VoiceError::Gone(_) => None,
        }
    }
}

/// The refusal a bus error name stands for (a client's side of the table).
#[must_use]
pub fn refusal_of_name(name: &str) -> Option<VoiceRefusedError> {
    name.strip_prefix(ERROR_PREFIX)
        .and_then(|_| VoiceRefusedError::from_error_name(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::DBusError;

    #[test]
    fn each_refusal_is_one_error_with_the_names_of_the_wire() {
        for refusal in VoiceRefusedError::ALL {
            let error = VoiceError::from(refusal);
            assert_eq!(error.refusal(), Some(refusal));
            let name = error.name();
            assert_eq!(name.as_str(), refusal.error_name());
            assert_eq!(refusal_of_name(name.as_str()), Some(refusal));
        }
        assert_eq!(refusal_of_name("org.freedesktop.DBus.Error.Failed"), None);
        assert_eq!(VoiceError::Malformed(String::new()).refusal(), None);
    }
}
