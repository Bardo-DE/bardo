//! Carrying out a tracker's outputs: socket messages, virtual device frames, and the actions that
//! are keys or are asked of the shell.

use super::socket::SocketHandle;
use mouse_gestures_core::action::plan;
use mouse_gestures_core::outputs::{Emitter, OutputError};
use mouse_gestures_core::socket::ToSocket;
use mouse_gestures_core::tracker::Out;

/// Performs every output, in order. Device write failures are logged and dropped: a missed
/// frame is better than a stalled reader. Returns whether any device write failed, which for
/// `Devices` means they were lost.
pub fn dispatch(outs: Vec<Out>, socket: &SocketHandle, emitter: &mut dyn Emitter) -> bool {
    outs.into_iter().map(|o| perform(&o, socket, emitter)).fold(
        false,
        |failed, result| match result {
            Ok(()) => failed,
            Err(e) => {
                eprintln!("bardo-mouse-gestures: output failed: {e}");
                true
            }
        },
    )
}

fn perform(out: &Out, socket: &SocketHandle, emitter: &mut dyn Emitter) -> Result<(), OutputError> {
    match out {
        Out::Host(m) => {
            socket.send(ToSocket::Msg(m.clone()));
            Ok(())
        }
        Out::Pad(f) => emitter.pad(f),
        Out::Key(k) => emitter.key(k),
        Out::Deferred(action) => {
            if let Some(host) = plan(action) {
                socket.send(ToSocket::Run(host));
            }
            Ok(())
        }
    }
}
