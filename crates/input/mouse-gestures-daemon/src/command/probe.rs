//! `mouse_gestures --check-uinput`: whether this machine can make the mouse gestures daemon's virtual devices.

use crate::daemon::uinput::{self, LiveSource, Origin};
use crate::error::Error;

/// Opens a session, makes the touchpad and lets go of both.
pub(super) fn check_uinput(origin: Origin) -> Result<(), Error> {
    uinput::probe(&LiveSource(origin))?;
    println!("mouse_gestures can make its virtual devices");
    Ok(())
}
