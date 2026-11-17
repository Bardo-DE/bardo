//! Refusals and their D-Bus error names, 1:1.

use serde::{Deserialize, Serialize};

/// The prefix of every error name of this interface.
pub const ERROR_PREFIX: &str = "org.bardo.Voice1.Error.";

/// Why a call was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum VoiceError {
    /// Voice is off.
    #[error("voice is disabled")]
    Disabled,
    /// First-use consent is pending.
    #[error("voice needs consent")]
    NeedsConsent,
    /// Busy.
    #[error("busy")]
    Busy,
    /// This caller may not.
    #[error("not allowed")]
    NotAllowed,
    /// No speech model.
    #[error("no model")]
    NoModel,
    /// No microphone.
    #[error("microphone unavailable")]
    MicUnavailable,
}

impl VoiceError {
    /// Every refusal.
    pub const ALL: [VoiceError; 6] = [
        VoiceError::Disabled,
        VoiceError::NeedsConsent,
        VoiceError::Busy,
        VoiceError::NotAllowed,
        VoiceError::NoModel,
        VoiceError::MicUnavailable,
    ];

    /// The full D-Bus error name.
    #[must_use]
    pub fn error_name(self) -> String {
        let variant = match self {
            VoiceError::Disabled => "Disabled",
            VoiceError::NeedsConsent => "NeedsConsent",
            VoiceError::Busy => "Busy",
            VoiceError::NotAllowed => "NotAllowed",
            VoiceError::NoModel => "NoModel",
            VoiceError::MicUnavailable => "MicUnavailable",
        };
        format!("{ERROR_PREFIX}{variant}")
    }

    /// The refusal a D-Bus error name stands for.
    #[must_use]
    pub fn from_error_name(name: &str) -> Option<VoiceError> {
        Self::ALL.into_iter().find(|r| r.error_name() == name)
    }
}
