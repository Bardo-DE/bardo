//! The lock screen: bardo-shell's own, on layer-shell-host's `ext-session-lock-v1` surfaces.
//!
//! See design/20 §1.9. bardo-shell locks on its own Lock action (the launcher row, the power menu, `bardo-shell lock`), on
//! logind's `Lock` for this session, and before sleep on `PrepareForSleep(true)`, holding a
//! logind delay inhibitor until the lock reports locked or `SLEEP_HOLD_MS` runs out. Never a
//! second lock while one holds.
//!
//! Which signals are bardo-shell's depends on the session (`shell_model::locker::Host`): in bardo-shell's own session every
//! trigger is; nested in another desktop's session (development) only
//! bardo-shell's own Lock action is, so bardo-shell never locks on another desktop's behalf.

mod backend;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::locker::LockerService;
use shell_model::service::ServiceIo;

impl Run for LockerService {
    type Kind = Self;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let bus = env.buses.system.clone();
        let runtime = env.dirs.runtime.clone();
        let desktop = env.current_desktop.clone();
        let session_id = env.session_id.clone();
        async move {
            backend::run(bus, runtime, desktop, session_id, io.state, io.commands).await;
            Ok(())
        }
    }
}
