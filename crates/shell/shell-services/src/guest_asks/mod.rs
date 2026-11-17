//! inferd (`org.bardo.Inference1`), as the shell hears it.
//!
//! When another of the person's computers asks to borrow this computer's models, inferd says so
//! and the shell asks "Let <name> use this computer's models?" in a notification with Allow and
//! Deny. A press is given to inferd as `AnswerGuest`. The notification is taken back when the
//! question is answered, here or in Settings.
//!
//! The service subscribes to `GuestAsks` and `GuestsChanged`, then reads `Guests` once for the
//! questions already waiting, and again whenever `org.bardo.Inference1` gets a new owner. Without
//! inferd the state stays empty.

mod backend;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::guest_asks::GuestAsksService;
use shell_model::service::ServiceIo;

pub use backend::GuestAsksLinks;

impl Run for GuestAsksService {
    type Kind = Self;
    type Deps = GuestAsksLinks;

    fn run(
        env: &Env,
        deps: Self::Deps,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let bus = env.buses.session.clone();
        async move { Box::pin(backend::run(bus, deps, io)).await }
    }
}
