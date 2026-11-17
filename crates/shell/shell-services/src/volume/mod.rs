//! The output volume: libpulse over pipewire-pulse, following the default sink.

mod backend;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::service::ServiceIo;
use shell_model::volume::VolumeService;

impl Run for VolumeService {
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let _ = env;
        async move {
            // The mainloop blocks: it runs on the blocking pool until `commands` closes, which
            // happens when every clone of the `Control` has dropped.
            let ServiceIo { state, commands } = io;
            backend::run(state, commands).await;
            Ok(())
        }
    }
}
