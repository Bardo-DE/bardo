//! The launcher's side of the run, over the bus: what the shell and the launcher do for the person.
//!
//! Open a conversation, record the turn with intentd, hand it to companiond, and watch the answer
//! object until it is done. Every step is a call or a signal on the private bus.

use crate::world::{GIVE_UP, World, within};
use account_core::AppName;
use action_client::{DbusTransport, Intents};
use action_core::{ContextKeep, Keep, TurnIn, TurnVia, UserTurn};
use action_dbus::{CompanionAnswerProxy, CompanionProxy};
use action_vocab::{Caller, Origin, WindowKey};
use action_wire::{SessionOpen, SessionOpened};
use companion_wire::{AnswerWire, AskWire};
use futures_util::StreamExt;
use provenance::{AgentRef, SpaceId, UnixSeconds};
use std::time::Duration;
use zbus::proxy::CacheProperties;
use zbus::zvariant::OwnedObjectPath;

/// Nothing of the screen is kept with a prompt.
#[must_use]
pub fn keep_nothing() -> ContextKeep {
    ContextKeep {
        query: Keep::Dropped,
        results: Keep::Dropped,
        selection: Keep::Dropped,
        window: Keep::Dropped,
    }
}

/// The shell's calls into intentd and companiond.
///
/// The connection owns `org.bardo.Shell`, which `actiond.toml` gives the launcher, confirm and
/// control roles and `companiond.toml` the right to speak for the person.
#[derive(Debug)]
pub struct Launcher {
    /// intentd, as the launcher.
    pub intents: Intents<DbusTransport>,
    connection: zbus::Connection,
    companion: CompanionProxy<'static>,
    patience: Duration,
    summoned: Option<AppName>,
}

impl Launcher {
    /// The launcher of `world`.
    ///
    /// # Panics
    ///
    /// Panics when the proxy to the bus cannot be made.
    pub async fn of(world: &World) -> Launcher {
        let connection = world.shell.clone();
        Launcher {
            intents: Intents::over(DbusTransport::new(connection.clone())),
            companion: within("the companion proxy", CompanionProxy::new(&connection))
                .await
                .expect("proxy"),
            connection,
            patience: GIVE_UP,
            summoned: None,
        }
    }

    /// The same launcher, asking from the window of `app`: the companion reads where the person
    /// is there (the thread they have open) when it resolves "this".
    #[must_use]
    pub fn summoned_from(self, app: AppName) -> Launcher {
        Launcher {
            summoned: Some(app),
            ..self
        }
    }

    /// The same launcher, waiting this long for each change of an answer instead of
    /// [`GIVE_UP`]: a real model on a cold engine is slower than a cassette.
    #[must_use]
    pub fn patient(self, patience: Duration) -> Launcher {
        Launcher { patience, ..self }
    }

    /// Opens a front conversation in the Space `work`.
    ///
    /// # Panics
    ///
    /// Panics when the bus call fails or its reply is not a `SessionOpened`.
    pub async fn open(&self) -> SessionOpened {
        let open = SessionOpen {
            space: SpaceId::parse("work").expect("space"),
            agent: AgentRef::Companion,
            parent: None,
            cwd: None,
            started_from: None,
            external: None,
            helper: None,
            acts_for: None,
        };
        let text = within(
            "Companion1.Open to answer",
            self.companion
                .open(&serde_json::to_string(&open).expect("json")),
        )
        .await
        .expect("Companion1.Open");
        serde_json::from_str(&text).expect("SessionOpened")
    }

    /// The person says `text`: intentd records the turn, companiond is asked. Returns the answer
    /// being watched, subscribed before the loop can say anything.
    ///
    /// # Panics
    ///
    /// Panics when a call to intentd or companiond fails.
    pub async fn say(&self, opened: &SessionOpened, text: &str) -> Answer {
        let id = Box::pin(within(
            "Session.Turn to answer",
            self.intents.session_turn(
                opened.session.clone(),
                TurnIn {
                    text: text.into(),
                    origin: Origin::Launcher,
                    keep: keep_nothing(),
                    via: TurnVia::Typed,
                    mode: None,
                },
            ),
        ))
        .await
        .expect("Session.Turn");
        let ask = AskWire {
            session: opened.session.clone(),
            turn: UserTurn {
                id,
                text: text.into(),
                at: UnixSeconds(1),
                from: Caller::launcher(),
                via: TurnVia::Typed,
                mode: None,
            },
            keep: keep_nothing(),
            parent_window: WindowKey::parse("w1").expect("window"),
            app: self.summoned.clone(),
        };
        let path: OwnedObjectPath = within(
            "Companion1.Ask to answer",
            self.companion.ask(
                &serde_json::to_string(&ask).expect("json"),
                &std::collections::HashMap::default(),
            ),
        )
        .await
        .expect("Companion1.Ask");
        Answer::watch(&self.connection, path, self.patience).await
    }
}

/// How many times its patience an answer may take in all, however often it changes.
const TOTAL_PATIENCES: u32 = 4;

/// One answer object, watched.
pub struct Answer {
    patience: Duration,
    proxy: CompanionAnswerProxy<'static>,
    updates: std::pin::Pin<Box<dyn futures_util::Stream<Item = String> + Send>>,
}

impl std::fmt::Debug for Answer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Answer")
    }
}

impl Answer {
    async fn watch(
        connection: &zbus::Connection,
        path: OwnedObjectPath,
        patience: Duration,
    ) -> Answer {
        let proxy = within(
            "the answer proxy",
            CompanionAnswerProxy::builder(connection)
                .path(path)
                .expect("answer path")
                .cache_properties(CacheProperties::No)
                .build(),
        )
        .await
        .expect("answer proxy");
        // Subscribed before the first `View` is read, so a change between the two is replayed.
        let updates = within("the Updated subscription", proxy.receive_updated())
            .await
            .expect("Updated signal")
            .filter_map(|signal| async move { signal.args().ok().map(|a| a.view().to_string()) });
        Answer {
            patience,
            proxy,
            updates: Box::pin(updates),
        }
    }

    /// The answer as it stands now.
    ///
    /// # Panics
    ///
    /// Panics when the call fails or its reply is not an `AnswerWire`.
    pub async fn view(&self) -> AnswerWire {
        let text = within("Answer.View to answer", self.proxy.view())
            .await
            .expect("Answer.View");
        serde_json::from_str(&text).expect("AnswerWire")
    }

    /// Every distinct view the answer takes, in order, until `stop` says one is the last. The
    /// initial view and every `Updated` payload count, so a phase that passes quickly (a sheet
    /// answered at once) is still in the list.
    ///
    /// # Panics
    ///
    /// Panics when the answer does not settle (see [`Answer::try_history_until`]).
    pub async fn history_until(&mut self, stop: impl Fn(&AnswerWire) -> bool) -> Vec<AnswerWire> {
        match self.try_history_until(stop).await {
            Ok(seen) => seen,
            Err(seen) => panic!("the answer never settled; saw {seen:#?}"),
        }
    }

    /// [`Answer::history_until`] that hands back what it saw, as the error, when nothing
    /// changed for the answer's patience, when the whole wait passed [`TOTAL_PATIENCES`] of it (an
    /// answer that keeps changing without coming to rest), or when the signal stream ended.
    ///
    /// # Errors
    ///
    /// Returns everything the answer showed as the error when it stopped changing for the answer's
    /// patience, when the whole wait ran out, or when the signal stream ended before `stop` was
    /// satisfied.
    pub async fn try_history_until(
        &mut self,
        stop: impl Fn(&AnswerWire) -> bool,
    ) -> Result<Vec<AnswerWire>, Vec<AnswerWire>> {
        let mut seen: Vec<AnswerWire> = Vec::new();
        let take = |view: AnswerWire, seen: &mut Vec<AnswerWire>| {
            let last = stop(&view);
            if seen.last() != Some(&view) {
                seen.push(view);
            }
            last
        };
        if take(self.view().await, &mut seen) {
            return Ok(seen);
        }
        let last_moment = tokio::time::Instant::now() + self.patience * TOTAL_PATIENCES;
        loop {
            let each = (tokio::time::Instant::now() + self.patience).min(last_moment);
            let Ok(Some(next)) = tokio::time::timeout_at(each, self.updates.next()).await else {
                return Err(seen);
            };
            let Ok(view) = serde_json::from_str::<AnswerWire>(&next) else {
                return Err(seen);
            };
            if take(view, &mut seen) {
                return Ok(seen);
            }
        }
    }
}

/// What memoryd holds of the Spaces `work` and `desktop` as the shell reads it (the shell's
/// identity is this test process, placed in the fake proc root as the shell).
///
/// The recent entries, polled until every kind in `kinds` is among them (the audit kinds of
/// `intentd`'s `record.rs`). intentd drains its audit queue into memoryd on a timer, so there is no
/// event to wait on; the bound only turns a missing record into a failure.
///
/// # Panics
///
/// Panics when the entries do not include every kind in `kinds` within the wait.
pub async fn recorded(world: &World, kinds: &[&str]) -> Vec<memory_core::RecentEntry> {
    use memory_client::{DbusTransport, Memory};
    use memory_core::{BodyMode, RecentQuery, TrustFilter};
    let memory = Memory::over(DbusTransport::new(world.shell.clone()));
    let spaces = ["work", "desktop"].map(|s| SpaceId::parse(s).expect("space"));
    let seen = std::cell::RefCell::new(Vec::<String>::new());
    let poll = async {
        loop {
            let mut entries = Vec::new();
            for space in &spaces {
                let query = RecentQuery {
                    since: UnixSeconds(0),
                    kinds: vec![],
                    trust: TrustFilter::Any,
                    limit: base_ids::Count(100),
                    bodies: BodyMode::Json,
                };
                entries.extend(
                    memory
                        .recent(space.clone(), query)
                        .await
                        .unwrap_or_default(),
                );
            }
            *seen.borrow_mut() = entries
                .iter()
                .map(|e| e.summary.kind.as_str().to_owned())
                .collect();
            if kinds
                .iter()
                .all(|k| entries.iter().any(|e| e.summary.kind.as_str() == *k))
            {
                return entries;
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    };
    tokio::time::timeout(GIVE_UP, poll)
        .await
        .unwrap_or_else(|_| {
            panic!(
                "memoryd never held {kinds:?}; it holds {:?}\n{}",
                seen.borrow(),
                world.logs()
            )
        })
}
