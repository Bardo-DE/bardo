//! The bar clock: a task that sleeps to each minute boundary and publishes the local time.

mod backend;

use std::sync::Arc;

use shell_model::clock::ClockMinute;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::clock::ClockService;
use shell_model::service::ServiceIo;

impl Run for ClockService {
    type Deps = ();

    fn initial(env: &Env, (): &()) -> ClockMinute {
        ClockMinute(env.clock.now().datetime())
    }

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let clock = Arc::clone(&env.clock);
        async move {
            backend::run(io.state, io.commands, clock).await;
            Ok(())
        }
    }
}
