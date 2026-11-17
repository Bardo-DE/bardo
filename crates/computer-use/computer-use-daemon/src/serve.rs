//! The computer-use daemon on the bus: `org.bardo.ComputerUse1` for the shell, the planner and the router, and
//! `org.bardo.IntentProvider1` as `org.bardo.ComputerUse` with the one action `computeruse.run.start`.

use crate::bus::ManagerObject;
use crate::provider::ComputerUseProvider;
use crate::reach::ReachObject;
use crate::relay::relay;
use crate::start::{answer_open_apps, starts};
use account_core::AppName;
use action_core::{ContextSnapshot, Here, Selection, TextTarget, Visible, WindowPrivacy};
use action_dbus::PROVIDER_PATH;
use action_dbus::callers::{CallerTable, Identity};
use action_provider::{ContextSource, SummonTarget};
use action_wire::{SummonAnswer, SummonOrigin, SummonSerial};
use base_ids::Count;
use computer_use_dbus::{COMPUTER_USE_BUS, COMPUTER_USE_PATH};
use computer_use_host::Supervisor;
use computer_use_run::Seams;
use futures_util::StreamExt;
use provenance::{Label, Labelled};
use std::sync::Arc;
use tokio::task::JoinHandle;

/// Why the daemon stopped.
#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    /// The session bus could not be reached or the name could not be taken.
    #[error("bus: {0}")]
    Bus(String),
}

fn bus<E: std::fmt::Display>(error: E) -> ServeError {
    ServeError::Bus(error.to_string())
}

/// The computer-use daemon shows the app's context to nobody: it has no windows of its own.
struct NoContext;

impl ContextSource for NoContext {
    fn snapshot(&self) -> ContextSnapshot {
        ContextSnapshot {
            app: AppName::parse("org.bardo.ComputerUse").expect("an app name"),
            window: Labelled::new(String::new(), Label::trusted_user()),
            here: Here::Nowhere,
            selection: Selection::Nothing,
            visible: Visible {
                kind: None,
                items: Vec::new(),
                total: Count(0),
            },
            text_target: TextTarget::None,
            privacy: WindowPrivacy::Private,
        }
    }
}

/// Nor is there a prompt to summon in it.
struct NoSummon;

impl SummonTarget for NoSummon {
    fn summon(&self, _serial: SummonSerial, _origin: SummonOrigin) -> SummonAnswer {
        SummonAnswer::Declined
    }
}

/// The computer-use daemon on a connection: its objects are up and its names are owned while this lives.
#[derive(Debug)]
pub struct Serving {
    tasks: Vec<JoinHandle<()>>,
}

impl Drop for Serving {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

/// Who the provider answers: intentd by the system's `/proc` in the daemon, or whom a test's
/// identity says is intentd.
enum Answering {
    IntentdOnly,
    Told(Arc<Identity>),
}

/// Serves the computer-use daemon's two interfaces on `connection`, naming callers by `table`.
///
/// They are `org.bardo.ComputerUse1` (`Manager` at `/org/bardo/ComputerUse1`, a `Run` per live run at
/// `/org/bardo/ComputerUse1/run/<id>`) and `org.bardo.IntentProvider1` as `org.bardo.ComputerUse`. Each caller's role is its process's, named by `table` (`computer-use-callers.toml`) over
/// the system's `/proc`, never a bus name it owns; the provider answers intentd's unit alone.
/// Returns once the names are ours; the connection keeps serving until the returned guard is
/// dropped.
///
/// # Errors
///
/// Returns [`ServeError::Bus`] when an object cannot be served on `connection` or the bus name
/// cannot be taken (it is already owned).
pub async fn serve_on<S: Seams>(
    connection: &zbus::Connection,
    supervisor: Arc<Supervisor<S>>,
    table: CallerTable,
) -> Result<Serving, ServeError> {
    let who = Arc::new(Identity::on_system(connection.clone(), table));
    serve_as(connection, supervisor, who, Answering::IntentdOnly).await
}

/// [`serve_on`] with callers named by `who`, the provider answering the process `who` names as
/// intentd.
///
/// This is the daemon's way in, where `who` reads the system's `/proc` (or a test build's fixture
/// tree, which the provider must read too, or intentd is a stranger to it).
///
/// # Errors
///
/// Returns [`ServeError::Bus`] when an object cannot be served on `connection` or the bus name
/// cannot be taken (it is already owned).
pub async fn serve_on_as<S: Seams>(
    connection: &zbus::Connection,
    supervisor: Arc<Supervisor<S>>,
    who: Identity,
) -> Result<Serving, ServeError> {
    let who = Arc::new(who);
    serve_as(
        connection,
        supervisor,
        Arc::clone(&who),
        Answering::Told(who),
    )
    .await
}

/// [`serve_on`] with callers named by `who` (a test's fixture proc root and introductions), which
/// also says who the provider takes for intentd.
///
/// # Errors
///
/// Returns [`ServeError::Bus`] when an object cannot be served on `connection` or the bus name
/// cannot be taken (it is already owned).
pub async fn serve_on_with<S: Seams>(
    connection: &zbus::Connection,
    supervisor: Arc<Supervisor<S>>,
    who: Arc<Identity>,
) -> Result<Serving, ServeError> {
    serve_as(
        connection,
        supervisor,
        Arc::clone(&who),
        Answering::Told(who),
    )
    .await
}

async fn serve_as<S: Seams>(
    connection: &zbus::Connection,
    supervisor: Arc<Supervisor<S>>,
    who: Arc<Identity>,
    answering: Answering,
) -> Result<Serving, ServeError> {
    // Subscribed before any name is taken, so no run is missed.
    let events = supervisor.subscribe();
    connection
        .object_server()
        .at(
            COMPUTER_USE_PATH,
            ManagerObject {
                supervisor: Arc::clone(&supervisor),
                who: Arc::clone(&who),
            },
        )
        .await
        .map_err(bus)?;
    // `org.bardo.IntentReach1` beside `IntentProvider1`: the live value of the one action.
    connection
        .object_server()
        .at(
            PROVIDER_PATH,
            ReachObject {
                supervisor: Arc::clone(&supervisor),
            },
        )
        .await
        .map_err(bus)?;
    let (provider, queues) =
        ComputerUseProvider::starting(supervisor.settings().enabled).map_err(bus)?;
    match answering {
        Answering::IntentdOnly => {
            action_provider::serve_on(connection, provider, NoContext, NoSummon)
                .await
                .map_err(bus)?;
        }
        Answering::Told(intentd) => {
            action_provider::serve_on_with(connection, provider, NoContext, NoSummon, intentd)
                .await
                .map_err(bus)?;
        }
    }
    dbus_kit::name::claim(connection, COMPUTER_USE_BUS)
        .await
        .map_err(bus)?;
    Ok(Serving {
        tasks: vec![
            tokio::spawn(relay(
                connection.clone(),
                Arc::clone(&supervisor),
                who,
                events,
            )),
            tokio::spawn(answer_open_apps(Arc::clone(&supervisor), queues.open_apps)),
            tokio::spawn(starts(supervisor, queues.starts)),
        ],
    })
}

/// Serves the computer-use daemon on the session bus until the bus goes away.
///
/// Callers are named by `table` (the caller reads `computer-use-callers.toml`, with
/// [`crate::read_callers`], from its own environment).
///
/// # Errors
///
/// Returns [`ServeError::Bus`] when the session bus cannot be reached, an object cannot be
/// served or the bus name cannot be taken.
pub async fn serve<S: Seams>(
    supervisor: Supervisor<S>,
    table: CallerTable,
) -> Result<(), ServeError> {
    let connection = zbus::Connection::session().await.map_err(bus)?;
    let _serving = serve_on(&connection, Arc::new(supervisor), table).await?;
    let mut messages = zbus::MessageStream::from(&connection);
    while messages.next().await.is_some() {}
    Ok(())
}
