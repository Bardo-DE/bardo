//! The work areas on the session bus: `org.bardo.Outputs1`, the shell's answer to "how much of
//! the screen may my window use, and at what scale".
//!
//! - **Interface `org.bardo.Outputs1`** at `/org/bardo/Outputs1`.
//!   - Property `WorkAreas`, type `a(us(iiuu)(iiuu)u)`: per output its id, connector name,
//!     frame (x, y, width, height), work area (the frame less the bar's and the dock's exclusive
//!     zones, same order) and scale in 120ths (120 is 1x). Logical pixels.
//!   - Change signal: the standard `PropertiesChanged` for `WorkAreas`, sent whenever an output,
//!     a panel's height or the dock's reveal changes the list.
//! - Read-only: nothing can be set or called.
//! - **Claims the name at start**, unless [`Env::claims`](crate::env::Env) is `Passive` (a test's
//!   `start` never claims it).

mod backend;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::service::ServiceIo;
use shell_model::work_areas::WorkAreaService;

impl Run for WorkAreaService {
    type Kind = Self;
    type Deps = ();

    fn run(
        env: &Env,
        _deps: (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let bus = env.buses.session.clone();
        let claims = env.claims;
        async move {
            backend::run(bus, claims, io).await;
            Ok(())
        }
    }
}
