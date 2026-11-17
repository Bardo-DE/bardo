//! The loop: keep the published list current from the daemon's `Publish`, claiming the bus name
//! at the start (unless the [`Env`](crate::env::Env) leaves names alone) and saying when the list
//! changes.

use shell_model::work_areas::{WorkAreaCommand, WorkAreas};
use tokio::sync::watch;
use zbus::Connection;

use super::object::OutputsObject;
use crate::env::{Bus, NameClaims};
use crate::serve;
use crate::service::ServiceIo;
use crate::work_areas::WorkAreaService;
use shell_dbus::{OUTPUTS_BUS, OUTPUTS_PATH};

pub(crate) async fn run(bus: Bus, claims: NameClaims, io: ServiceIo<WorkAreaService>) {
    let ServiceIo {
        state,
        mut commands,
    } = io;
    let seat = match claims {
        NameClaims::Claim => claim(&bus, &state).await,
        NameClaims::Passive => None,
    };
    while let Some(WorkAreaCommand::Publish(areas)) = commands.recv().await {
        let changed = state.send_if_modified(|published| {
            let changed = published.areas != areas;
            if changed {
                published.areas = areas;
            }
            changed
        });
        if let (true, Some(connection)) = (changed, &seat) {
            announce(connection).await;
        }
    }
}

async fn claim(bus: &Bus, state: &watch::Sender<WorkAreas>) -> Option<Connection> {
    let claimed = serve::claim(bus, OUTPUTS_BUS, async |connection| {
        let object = OutputsObject {
            state: state.subscribe(),
        };
        connection
            .object_server()
            .at(OUTPUTS_PATH, object)
            .await
            .map(|_| ())
            .map_err(serve::refused)
    })
    .await;
    let (link, seat) = serve::held(claimed);
    state.send_modify(|published| published.link = link);
    seat
}

/// Send `PropertiesChanged` for `WorkAreas`.
async fn announce(connection: &Connection) {
    let Ok(object) = connection
        .object_server()
        .interface::<_, OutputsObject>(OUTPUTS_PATH)
        .await
    else {
        return;
    };
    let emitter = object.signal_emitter();
    if let Err(error) = object.get().await.work_areas_changed(emitter).await {
        eprintln!("bardo-shell: work areas: announcing the change failed: {error}");
    }
}
