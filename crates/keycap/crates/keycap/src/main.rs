//! The `keycap` command: the daemon and its tools.

use std::process::ExitCode;

fn main() -> ExitCode {
    abort_on_panic();
    keycap::command::run(std::env::args())
}

/// Any panic, on any thread, takes the whole process down, as `panic = "abort"` in the old
/// repo's release profile did: a half-working daemon looks alive to systemd, a dead one is
/// restarted. The default hook still prints the message first. (A profile cannot set this per
/// binary in the monorepo, and other crates rely on catching panics.)
fn abort_on_panic() {
    let report = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        report(info);
        std::process::abort();
    }));
}
