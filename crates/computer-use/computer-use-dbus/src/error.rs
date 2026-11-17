//! `org.bardo.ComputerUse1.Error.<Variant>`: a control call refused. Mapped 1:1 from `ControlRefusal`.

use computer_use_bus::ControlRefusal;

/// The errors the control interfaces return.
#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "org.bardo.ComputerUse1.Error")]
pub enum ComputerUseError {
    /// zbus's own failures.
    #[zbus(error)]
    ZBus(zbus::Error),
    /// The caller is not on the allow-list for this member.
    NotPermitted(String),
    /// The run is not in a state where this applies.
    WrongState(String),
    /// No such question.
    UnknownQuestion(String),
    /// `computer_use.enabled` is off.
    Disabled(String),
    /// No such window.
    WindowUnknown(String),
    /// Another run holds the window.
    LeaseHeld(String),
    /// The backend cannot run this mode.
    ModeUnsupported(String),
    /// The task body is malformed or in a vocabulary this build does not read.
    BadTask(String),
    /// No computer-use model can serve a run.
    ModelUnavailable(String),
}

impl ComputerUseError {
    /// The refusal this error carries, or `None` for a bus failure.
    #[must_use]
    pub fn refusal(&self) -> Option<ControlRefusal> {
        match self {
            ComputerUseError::ZBus(_) => None,
            ComputerUseError::NotPermitted(_) => Some(ControlRefusal::NotPermitted),
            ComputerUseError::WrongState(_) => Some(ControlRefusal::WrongState),
            ComputerUseError::UnknownQuestion(_) => Some(ControlRefusal::UnknownQuestion),
            ComputerUseError::Disabled(_) => Some(ControlRefusal::Disabled),
            ComputerUseError::WindowUnknown(_) => Some(ControlRefusal::WindowUnknown),
            ComputerUseError::LeaseHeld(_) => Some(ControlRefusal::LeaseHeld),
            ComputerUseError::ModeUnsupported(_) => Some(ControlRefusal::ModeUnsupported),
            ComputerUseError::BadTask(_) => Some(ControlRefusal::BadTask),
            ComputerUseError::ModelUnavailable(_) => Some(ControlRefusal::ModelUnavailable),
        }
    }
}

impl From<ControlRefusal> for ComputerUseError {
    fn from(refusal: ControlRefusal) -> Self {
        let text = String::new();
        match refusal {
            ControlRefusal::NotPermitted => ComputerUseError::NotPermitted(text),
            ControlRefusal::WrongState => ComputerUseError::WrongState(text),
            ControlRefusal::UnknownQuestion => ComputerUseError::UnknownQuestion(text),
            ControlRefusal::Disabled => ComputerUseError::Disabled(text),
            ControlRefusal::WindowUnknown => ComputerUseError::WindowUnknown(text),
            ControlRefusal::LeaseHeld => ComputerUseError::LeaseHeld(text),
            ControlRefusal::ModeUnsupported => ComputerUseError::ModeUnsupported(text),
            ControlRefusal::BadTask => ComputerUseError::BadTask(text),
            ControlRefusal::ModelUnavailable => ComputerUseError::ModelUnavailable(text),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::DBusError;

    #[test]
    fn error_names_cover_control_refusal() {
        for refusal in ControlRefusal::ALL {
            let error = ComputerUseError::from(refusal);
            assert_eq!(error.refusal(), Some(refusal));
            assert_eq!(
                error.name().as_str(),
                format!("org.bardo.ComputerUse1.Error.{refusal:?}"),
                "{refusal:?}"
            );
        }
    }
}
