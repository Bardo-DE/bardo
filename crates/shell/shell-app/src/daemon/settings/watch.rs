//! The settings file's watch: each settled change becomes a `ShellMsg::Settings` for the host
//! thread, which publishes it.

use layer_shell_host::HostHandle;
use shell_settings::ShellFile;
use tokio::runtime::Runtime;
use ui_blitz::TokioSpawner;
use settings_store_fs::{Store, WatchState};

use super::store::report_shell;
use crate::daemon::msg::ShellMsg;

/// Watch `store`'s `settings.toml` on `runtime`, sending each settled file to `host`. A directory
/// that cannot be watched is logged and the settings then change only through `bardo-shell reload`.
pub fn spawn_watch(runtime: &Runtime, store: &Store, host: HostHandle<ShellMsg>) {
    let mut watch = store.watch::<ShellFile>(&TokioSpawner::on(runtime.handle().clone()));
    if let WatchState::Blind { reason } = watch.state() {
        eprintln!("bardo-shell: watching the settings: {reason}");
        return;
    }
    runtime.spawn(async move {
        while let Some(loaded) = watch.changed().await {
            report_shell(&loaded);
            if host
                .send(ShellMsg::Settings(Box::new(loaded.value)))
                .is_err()
            {
                break;
            }
        }
    });
}
