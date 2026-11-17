//! Next Input Source (the Mac's Control-Space) on the session bus.
//!
//! The running input-method framework is looked for when the action fires, never assumed: fcitx5's
//! `org.fcitx.Fcitx.Controller1` is switched to the entry after the current one in its current
//! group ([`shell_model::input_source::next_of`]). IBus keeps its own bus, found through an address
//! file (or `IBUS_ADDRESS`, read at the edge as [`crate::env::IbusRoute`]); its active engines are
//! cycled the same way. No framework at all is a typed refusal. The outcome is the service's
//! state.

mod backend;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::input_source::InputSourceService;
use shell_model::service::ServiceIo;

impl Run for InputSourceService {
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        backend::run(
            env.buses.session.clone(),
            env.ibus.clone(),
            io.state,
            io.commands,
        )
    }
}
