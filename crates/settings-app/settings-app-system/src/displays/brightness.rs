//! The brightness sliders' side of the shell's bus: `org.bardo.Displays1`, which lists the
//! displays whose brightness bardo-shell can read and sets one. The owner of the name must hold the
//! shell's role before it is read or told anything (`live::pinned`), and a call goes to the owner
//! that was confirmed. Nothing is started: a shell that is not there has no sliders.

use crate::bus::{BusSource, because, connect, pinned, said, watch_on};
use crate::deadline::{Deadline, answered, peer};
use crate::ticks::Ticks;
use settings_app_model::{Absence, Brightness, BrightnessList, OutputId};
use shell_dbus::{DISPLAYS_BUS, Displays1Proxy};
use zbus::Message;

const COULD_NOT_SET: &str = "The brightness can\u{2019}t be changed right now.";

/// The displays that have a brightness now.
pub async fn read(bus: &BusSource) -> Result<BrightnessList, Absence> {
    let running = pinned(bus, DISPLAYS_BUS)
        .await
        .map_err(because(Absence::NotRunning))?;
    let proxy = Displays1Proxy::builder(&running.connection)
        .destination(running.destination())
        .map_err(because(Absence::Failed))?
        .build()
        .await
        .map_err(because(Absence::Failed))?;
    let rows = answered(Deadline::READ, peer::DISPLAYS, proxy.brightness())
        .await
        .ok_or(Absence::Failed)?
        .map_err(because(Absence::Failed))?;
    Ok(BrightnessList::of_wire(&rows))
}

/// Sets `output` to `level`; the words to say when it could not.
pub async fn set(bus: &BusSource, output: &OutputId, level: Brightness) -> Result<(), String> {
    let running = pinned(bus, DISPLAYS_BUS)
        .await
        .map_err(said(COULD_NOT_SET))?;
    let proxy = Displays1Proxy::builder(&running.connection)
        .destination(running.destination())
        .map_err(said(COULD_NOT_SET))?
        .build()
        .await
        .map_err(said(COULD_NOT_SET))?;
    answered(
        Deadline::ACTION,
        peer::DISPLAYS,
        proxy.set_brightness(&output.0, u32::from(level.0)),
    )
    .await
    .ok_or_else(|| COULD_NOT_SET.to_owned())?
    .map_err(said(COULD_NOT_SET))
}

/// A tick each time the shell says the levels changed, or its name changes hands.
pub async fn changes(bus: &BusSource) -> Result<Ticks, Absence> {
    let connection = connect(bus).await.map_err(because(Absence::NotRunning))?;
    watch_on(&connection, DISPLAYS_BUS, is_levels).await
}

/// Whether `message` is the shell's word that the levels changed.
fn is_levels(message: &Message) -> bool {
    message
        .header()
        .interface()
        .is_some_and(|interface| interface.as_str() == "org.bardo.Displays1")
}
