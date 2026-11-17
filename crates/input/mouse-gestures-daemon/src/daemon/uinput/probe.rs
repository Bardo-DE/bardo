//! `mouse_gestures --check-uinput`: can this machine make the mouse gestures daemon's virtual devices?

use super::emitter::UinputEmitter;
use crate::outputs::OutputError;
use input_broker_client::DeviceSource;

/// Opens a session on `source` and makes the touchpad, then lets go of both: the same first
/// steps the daemon takes, so a pass here means the daemon will get its devices.
pub fn probe<D: DeviceSource>(source: &D) -> Result<(), OutputError> {
    UinputEmitter::create(source, &[]).map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;
    use input_broker_client::Reason;
    use input_broker_client::fake::{FakeSource, OpenMode};

    #[test]
    fn the_check_makes_one_device_and_reports_a_refusal() {
        let source = FakeSource::new();
        probe(&source).unwrap();
        assert_eq!(source.state().created.len(), 1);

        source.state().refuse_next = Some(Reason::NotAllowed);
        assert!(matches!(probe(&source), Err(OutputError::Cut(_))));

        source.state().open_mode = OpenMode::Absent;
        assert!(matches!(probe(&source), Err(OutputError::Unavailable(_))));
    }
}
