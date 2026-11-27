//! The computers over inferd, through inferd's client: the places, the Tailscale candidates, the
//! guests and the changes. The engines signal (a computer answering or not) comes from its proxy.

use crate::bus::{Pinned, ToAbsence, absence_of, because, inferd, inferd_owners, read_once};
use account_dbus::BusStream;
use base_time::deadline::within;
use inference_client::{
    ClientError, ComputerError, ComputerReach, DbusTransport, Inference, NewComputer,
    NewComputerModel, TransportError,
};
use inference_dbus::inference::InferenceProxy;
use inference_tailnet::ComputerCandidate;
use settings_app_link::bus::{BusSource, Daemon};
use settings_app_link::computers::{Change, ChangeFailure, ComputersLink};
use settings_app_link::{Deadline, answered, peer};
use settings_app_link::{Ticker, Ticks};
use settings_app_model::{
    Absence, ComputerRow, ComputersFound, ComputersReason, ServerDraft, ServerModel, ServerReach,
    Work, picker_row_is_choice, server_models,
};
use std::path::Path;

/// Why a call was refused, as the pane knows it.
fn reason_of(error: &ClientError) -> ComputersReason {
    match error {
        ClientError::Transport(TransportError::Computer { reason, .. }) => match reason {
            ComputerError::BadName => ComputersReason::BadName,
            ComputerError::NoModels => ComputersReason::NoModels,
            ComputerError::TooMany => ComputersReason::TooMany,
            ComputerError::AlreadyThere => ComputersReason::AlreadyThere,
            ComputerError::UnknownModel => ComputersReason::UnknownModel,
            ComputerError::ModelNotUsable => ComputersReason::ModelNotUsable,
            ComputerError::ModelTaken => ComputersReason::ModelTaken,
            ComputerError::BadAddress => ComputersReason::BadAddress,
            ComputerError::BadKey => ComputersReason::BadKey,
            ComputerError::NotSaved => ComputersReason::NotSaved,
            ComputerError::NotThere => ComputersReason::NotThere,
            ComputerError::AddedByHand => ComputersReason::AddedByHand,
            ComputerError::Unavailable => ComputersReason::Unavailable,
            ComputerError::NotOnTailscale => ComputersReason::NotOnTailscale,
            ComputerError::NotAnswering => ComputersReason::NotAnswering,
            ComputerError::NotAsking => ComputersReason::NotAsking,
            ComputerError::TooManyAsking => ComputersReason::TooManyAsking,
            ComputerError::Other(_) => ComputersReason::Failed,
        },
        ClientError::Transport(TransportError::Denied(_)) => ComputersReason::NotAllowed,
        _ => ComputersReason::Failed,
    }
}

fn accounts(inferd: &Pinned) -> Result<Inference<DbusTransport>, Absence> {
    Ok(Inference::over(inferd.inference()?))
}

/// The person's computers and the guests, as inferd lists them now.
async fn read(bus: &BusSource) -> Result<ComputersFound, Absence> {
    let running = inferd(bus).await?;
    let accounts = accounts(&running)?;
    let asked = async {
        let places = accounts.places().await.map_err(absence_of)?;
        let guests = accounts.guests().await.map_err(absence_of)?;
        Ok::<_, Absence>((places, guests))
    };
    let (places, guests) = within(Deadline::READ, peer::ASSISTANT, asked)
        .await
        .map_err(because(Absence::Failed))??;
    Ok(ComputersFound {
        computers: places.iter().filter_map(ComputerRow::of).collect(),
        guests,
    })
}

/// The computers pane's failure for an absent service; the absence itself goes to the log.
fn unreached(why: Absence) -> ComputersReason {
    eprintln!("bardo-settings: computers: the service is {why:?}");
    ComputersReason::Failed
}

/// The Tailscale computers that can be added, or why they cannot be listed.
async fn candidates(bus: &BusSource) -> Result<Vec<ComputerCandidate>, ComputersReason> {
    let running = inferd(bus).await.map_err(unreached)?;
    let accounts = accounts(&running).map_err(unreached)?;
    answered(Deadline::READ, peer::ASSISTANT, accounts.candidates())
        .await
        .ok_or(ComputersReason::Failed)?
        .map_err(|e| reason_of(&e))
}

/// Adds the server. A key kept in a file is read now, and only now; a file that cannot be read
/// is told in the sheet's own words, and nothing is sent.
async fn add_server(bus: &BusSource, draft: ServerDraft) -> Result<(), ChangeFailure> {
    let refused = ChangeFailure::Refused;
    let key = draft
        .secret(|path: &Path| std::fs::read_to_string(path))
        .map_err(ChangeFailure::Fault)?;
    let reach = match draft.reach {
        ServerReach::Port(port) => ComputerReach::Port(port),
        ServerReach::Socket(path) => ComputerReach::Socket(path),
    };
    let served = NewComputerModel::new(draft.model, reach);
    let served = match key {
        Some(key) => served.with_key(key),
        None => served,
    };
    let computer = NewComputer::new(draft.name, vec![served]);
    let running = inferd(bus).await.map_err(|why| refused(unreached(why)))?;
    let accounts = accounts(&running).map_err(|why| refused(unreached(why)))?;
    answered(
        Deadline::ACTION,
        peer::ASSISTANT,
        accounts.add_computer(&computer),
    )
    .await
    .ok_or(refused(ComputersReason::Failed))?
    .map(drop)
    .map_err(|e| refused(reason_of(&e)))
}

/// The models a server may run, as inferd's picker lists them; none when it cannot be read.
async fn models(bus: &BusSource) -> Vec<ServerModel> {
    answered(
        Deadline::READ,
        peer::ASSISTANT,
        read_once(bus, &Daemon::inferd(), picker_row_is_choice),
    )
    .await
    .and_then(Result::ok)
    .map_or_default(|described| server_models(&described))
}

/// Makes one change; why not when it was refused.
async fn change(bus: &BusSource, change: Change) -> Result<(), ChangeFailure> {
    let refused = ChangeFailure::Refused;
    let change = match change {
        Change::Server(draft) => return add_server(bus, draft).await,
        other => other,
    };
    let running = inferd(bus).await.map_err(|why| refused(unreached(why)))?;
    let accounts = accounts(&running).map_err(|why| refused(unreached(why)))?;
    let done = answered(Deadline::ACTION, peer::ASSISTANT, async {
        match change {
            Change::Add(node) => accounts.add_tailnet_computer(&node).await.map(drop),
            Change::Remove(computer) => accounts.remove_computer(&computer).await,
            Change::Answer(node, answer) => accounts.answer_guest(&node, answer).await,
            Change::Forget(node) => accounts.forget_guest(&node).await,
            Change::Server(_) => Ok(()),
        }
    })
    .await
    .ok_or(refused(ComputersReason::Failed))?;
    done.map_err(|e| refused(reason_of(&e)))
}

/// Tells `out` each time `stream` yields, until the stream ends or nobody listens.
pub async fn relay(mut stream: impl BusStream<Item = impl Send> + Unpin, out: Ticker) {
    while std::future::poll_fn(|cx| std::pin::Pin::new(&mut stream).poll_next(cx))
        .await
        .is_some()
    {
        if out.tick().is_err() {
            return;
        }
    }
}

/// The signals that say the lists changed: the guests, and the engines (a computer answering or
/// not). Each is relayed by a task that ends with the watch (dropping it aborts them), and ticks
/// that come while one waits are one tick. Subscribes only when inferd is on the bus.
async fn watch(bus: &BusSource) -> Result<Ticks, Absence> {
    {
        let running = inferd(bus).await?;
        let accounts = accounts(&running)?;
        let subscribed = async {
            let guests = accounts.watch_guests().await.map_err(absence_of)?;
            let engines = InferenceProxy::builder(&running.connection)
                .destination(running.destination())
                .map_err(|e| e.absence())?
                .build()
                .await
                .map_err(|e| e.absence())?
                .receive_engines_changed()
                .await
                .map_err(|e| e.absence())?;
            Ok::<_, Absence>((guests, engines))
        };
        let (guests, engines) = within(Deadline::READ, peer::ASSISTANT, Box::pin(subscribed))
            .await
            .map_err(because(Absence::Failed))??;
        let (ticker, ticks) = Ticks::channel();
        let ticks = ticks
            .with_relay(tokio::spawn(relay(guests, ticker.clone())))
            .with_relay(tokio::spawn(relay(engines, ticker)));
        Ok(ticks)
    }
}

/// inferd on the bus it is built over.
#[derive(Debug)]
pub struct OnBus(pub BusSource);

impl ComputersLink for OnBus {
    fn read(&self) -> Work<'_, Result<ComputersFound, Absence>> {
        Box::pin(read(&self.0))
    }

    fn candidates(&self) -> Work<'_, Result<Vec<ComputerCandidate>, ComputersReason>> {
        Box::pin(candidates(&self.0))
    }

    fn models(&self) -> Work<'_, Vec<ServerModel>> {
        Box::pin(models(&self.0))
    }

    fn change(&self, change: Change) -> Work<'_, Result<(), ChangeFailure>> {
        Box::pin(self::change(&self.0, change))
    }

    fn changes(&self) -> Work<'_, Result<Ticks, Absence>> {
        Box::pin(watch(&self.0))
    }

    fn owner_changes(&self) -> Work<'_, Result<Ticks, Absence>> {
        Box::pin(inferd_owners(&self.0))
    }
}
