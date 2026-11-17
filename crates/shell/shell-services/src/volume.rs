//! The output volume: libpulse over pipewire-pulse, following the default sink.

mod backend;

use crate::env::{Env, SoundRoute};
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::service::ServiceIo;
use shell_model::volume::VolumeService;

impl Run for VolumeService {
    type Kind = Self;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let sound = env.sound;
        async move {
            let ServiceIo {
                state,
                mut commands,
            } = io;
            match sound {
                // The mainloop blocks: it runs on the blocking pool until `commands` closes,
                // which happens when every clone of the `Control` has dropped.
                SoundRoute::Server => backend::run(state, commands).await,
                // No sound server to tell: the commands go nowhere until they stop coming.
                SoundRoute::Absent => while commands.recv().await.is_some() {},
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shell_model::volume::VolumeCommand;
    use tokio::sync::{mpsc, watch};

    #[tokio::test]
    async fn absent_sound_drops_commands_and_ends_with_them() {
        let scratch = std::env::temp_dir();
        let env = Env::isolated(&scratch);
        assert_eq!(env.sound, SoundRoute::Absent);
        let (state, _published) = watch::channel(Default::default());
        let (control, commands) = mpsc::unbounded_channel();
        control
            .send(VolumeCommand::ToggleMute)
            .expect("the receiver is alive");
        drop(control);
        VolumeService::run(&env, (), ServiceIo { state, commands })
            .await
            .expect("an absent sound server is not an error");
    }
}
