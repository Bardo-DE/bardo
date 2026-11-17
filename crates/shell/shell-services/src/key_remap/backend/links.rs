//! What the key remap daemon service reads and drives, and where it connects.

use std::path::PathBuf;

use shell_model::apps::AppIndex;
use shell_model::notifications::{NotesState, NotificationsCommand};
use shell_model::windows::WindowModel;
use tokio::sync::watch;

use crate::env::{Bus, BusRoute, Env, UnitWait};
use shell_model::service::Control;

/// The other services the key remap daemon reads and drives.
#[derive(Debug, Clone)]
pub struct KeyRemapLinks {
    /// Which window has the focus.
    pub windows: watch::Receiver<WindowModel>,
    /// The desktop entries that tell which profile the focused window wants.
    pub apps: watch::Receiver<AppIndex>,
    /// Where the notices about the key remap daemon are posted and withdrawn.
    pub notifications: Control<NotificationsCommand>,
    /// What the notifications service holds, to find the notice already shown.
    pub notes: watch::Receiver<NotesState>,
}

/// Where the service connects.
#[derive(Debug, Clone)]
pub(crate) struct Reach {
    /// The key remap daemon's socket; `None` and the service never connects.
    pub(crate) socket: Option<PathBuf>,
    /// The system bus, where systemd answers about the unit; `None` when there is no such bus.
    pub(crate) system: Option<Bus>,
    /// When the unit is asked about.
    pub(crate) unit_wait: UnitWait,
    /// The key remap daemon's folder under the config directory.
    pub(crate) config_dir: PathBuf,
}

impl Reach {
    /// `env`'s the key remap daemon socket, system bus and config directory.
    pub(crate) fn of(env: &Env) -> Reach {
        let system = match env.buses.system.route() {
            BusRoute::Absent => None,
            BusRoute::Usual | BusRoute::Address(_) => Some(env.buses.system.clone()),
        };
        Reach {
            socket: env.key_remap_socket.clone(),
            system,
            unit_wait: env.key_remap_unit_wait,
            config_dir: env.dirs.key_remap_config(),
        }
    }
}
