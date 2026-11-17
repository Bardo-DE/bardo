//! Scripted seams and a raw client for the session server's tests. The test plays the engine: a
//! started turn hands the test a sender, and the turn yields whatever the test pushes.

use account_capability::ModelId;
use account_capability::{Locality, Tier};
use account_core::AccountId;
use account_core::DataClass;
use account_match::Need;
use account_wire::{decode_frame, encode_frame};
use base_ids::Tokens;
use inference_core::{
    AudioFrame, ClientFrame, InferEvent, InferRefusedError, InferReply, InferRequest, ModelRef,
    Readiness, ServedBy,
};
use inference_router::carried::Carried;
use inference_router::seams::{
    AuditSink, EngineFailed, EngineHost, Router, RunningTurn, TurnRunner, TurnStep,
};
use inference_router::session::{RouteDecision, SessionSpec};
use rustix::net::{SendAncillaryBuffer, SendAncillaryMessage, SendFlags, sendmsg};
use std::io::{IoSlice, Read, Seek};
use std::mem::MaybeUninit;
use std::os::fd::{AsFd, OwnedFd};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, Interest};
use tokio::net::UnixStream;
use tokio::sync::{Notify, mpsc};

pub fn served() -> ServedBy {
    ServedBy {
        account: AccountId::parse("local").expect("id"),
        model: ModelId::parse("qwen").expect("id"),
        locality: Locality::OnDevice,
    }
}

pub fn model() -> ModelRef {
    ModelRef {
        account: AccountId::parse("local").expect("id"),
        model: ModelId::parse("qwen").expect("id"),
    }
}

pub fn spec(need: Need, class: DataClass) -> SessionSpec {
    SessionSpec {
        need,
        class,
        tier: Tier::Balanced,
        usage: account_core::consent::Usage::Interactive,
    }
}

pub fn llm() -> Need {
    Need::Llm(account_match::LlmNeed::new(
        std::collections::BTreeSet::default(),
        Tokens(1),
    ))
}

/// A router with one fixed answer.
#[derive(Debug)]
pub struct FixedRouter(pub Result<RouteDecision, InferRefusedError>);

impl FixedRouter {
    pub fn ready() -> Self {
        Self(Ok(RouteDecision {
            served: served(),
            readiness: Readiness::Ready,
        }))
    }

    pub fn loading() -> Self {
        Self(Ok(RouteDecision {
            served: served(),
            readiness: Readiness::Loadable,
        }))
    }
}

impl Router for FixedRouter {
    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn route(&self, _spec: &SessionSpec) -> Result<RouteDecision, InferRefusedError> {
        self.0.clone()
    }
}

/// Engines the test starts by hand: `want` resolves when `go` is notified, or at once.
#[derive(Debug, Default)]
pub struct Engines {
    pub go: Option<Arc<Notify>>,
    pub fail: bool,
    pub released: Arc<Mutex<Vec<ModelRef>>>,
}

impl EngineHost for Engines {
    async fn want(&self, _model: ModelRef) -> Result<(), EngineFailed> {
        if let Some(go) = &self.go {
            go.notified().await;
        }
        if self.fail {
            Err(EngineFailed::unknown())
        } else {
            Ok(())
        }
    }

    fn release(&self, model: &ModelRef) {
        self.released.lock().expect("lock").push(model.clone());
    }
}

/// What a started turn gave the test.
#[derive(Debug)]
pub struct Started {
    pub request: InferRequest,
    /// The bytes of each descriptor that rode with the request.
    pub attachments: Vec<Vec<u8>>,
    /// Push what the engine says.
    pub tx: mpsc::UnboundedSender<TurnStep>,
}

#[derive(Debug, Default, Clone)]
pub struct Runner {
    pub started: Arc<Mutex<Vec<Started>>>,
    pub audio: Arc<Mutex<Vec<AudioFrame>>>,
    pub audio_ended: Arc<Mutex<usize>>,
}

#[derive(Debug)]
pub struct Turn {
    rx: mpsc::UnboundedReceiver<TurnStep>,
    runner: Runner,
}

impl TurnRunner for Runner {
    type Turn = Turn;

    fn start(&self, request: InferRequest, attachments: Vec<OwnedFd>) -> Turn {
        let (tx, rx) = mpsc::unbounded_channel();
        let contents = attachments
            .into_iter()
            .map(|fd| {
                let mut file = std::fs::File::from(fd);
                let mut bytes = Vec::new();
                file.rewind().expect("rewind");
                file.read_to_end(&mut bytes).expect("read");
                bytes
            })
            .collect();
        self.started.lock().expect("lock").push(Started {
            request,
            attachments: contents,
            tx,
        });
        Turn {
            rx,
            runner: self.clone(),
        }
    }

    fn start_heard(
        &self,
        chat: inference_core::ChatRequest,
        heard: inference_router::session::HeardAudio,
    ) -> Turn {
        self.audio.lock().expect("lock").extend(heard.frames);
        self.start(InferRequest::Chat(chat), Vec::new())
    }
}

impl RunningTurn for Turn {
    fn audio(&mut self, frame: AudioFrame) {
        self.runner.audio.lock().expect("lock").push(frame);
    }

    fn end_audio(&mut self) {
        *self.runner.audio_ended.lock().expect("lock") += 1;
    }

    async fn next(&mut self) -> TurnStep {
        match self.rx.recv().await {
            Some(step) => step,
            None => std::future::pending().await,
        }
    }
}

/// The replies the audit sink was told of, and what each turn's request carried.
#[derive(Debug, Default, Clone)]
pub struct Audit(
    pub Arc<Mutex<Vec<InferReply>>>,
    pub Arc<Mutex<Vec<Carried>>>,
);

impl AuditSink for Audit {
    fn record(
        &self,
        _spec: &SessionSpec,
        served: &ServedBy,
        reply: &InferReply,
        carried: &Carried,
    ) {
        assert_eq!(served, &self::served());
        self.0.lock().expect("lock").push(reply.clone());
        self.1.lock().expect("lock").push(*carried);
    }
}

/// The client's end of a session socket.
#[derive(Debug)]
pub struct Client {
    stream: UnixStream,
    inbox: Vec<u8>,
}

impl Client {
    pub fn new(stream: UnixStream) -> Self {
        Self {
            stream,
            inbox: Vec::new(),
        }
    }

    /// Writes a frame with descriptors on it (one `sendmsg`, as the real client does).
    pub async fn send(&mut self, frame: &ClientFrame, fds: &[OwnedFd]) {
        let bytes = encode_frame(frame).expect("encode");
        let sent = self
            .stream
            .async_io(Interest::WRITABLE, || {
                let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(16))];
                let mut control = SendAncillaryBuffer::new(&mut space);
                let borrowed: Vec<_> = fds.iter().map(AsFd::as_fd).collect();
                if !borrowed.is_empty() {
                    control.push(SendAncillaryMessage::ScmRights(&borrowed));
                }
                sendmsg(
                    &self.stream,
                    &[IoSlice::new(&bytes)],
                    &mut control,
                    SendFlags::NOSIGNAL,
                )
                .map_err(std::io::Error::from)
            })
            .await
            .expect("sendmsg");
        assert_eq!(sent, bytes.len(), "the test frames are small");
    }

    /// The next event, or `None` when the daemon closed the socket.
    pub async fn event(&mut self) -> Option<InferEvent> {
        tokio::time::timeout(base_testkit::GENEROUS, self.event_inner())
            .await
            .expect("an event or a close within the generous wait")
    }

    async fn event_inner(&mut self) -> Option<InferEvent> {
        loop {
            if let Ok(Some((envelope, used))) = decode_frame::<InferEvent>(&self.inbox) {
                self.inbox.drain(..used);
                return Some(envelope.body);
            }
            let mut chunk = [0_u8; 4096];
            match self.stream.read(&mut chunk).await {
                Ok(0) | Err(_) => return None,
                Ok(n) => self.inbox.extend_from_slice(&chunk[..n]),
            }
        }
    }

    /// Events up to and including the next `Finished`.
    pub async fn until_finished(&mut self) -> Vec<InferEvent> {
        let mut events = Vec::new();
        while let Some(event) = self.event().await {
            let done = matches!(event, InferEvent::Finished(_));
            events.push(event);
            if done {
                break;
            }
        }
        events
    }
}

/// Waits until `name` has an owner on the bus `client` is on (`owned`), or none (`!owned`). It
/// wakes on the bus's own `NameOwnerChanged`, not on the clock.
///
/// # Panics
/// When the owner does not change as asked within [`base_testkit::GENEROUS`].
pub async fn name_owner(client: &zbus::Connection, name: &str, owned: bool) {
    use futures_util::StreamExt;
    let dbus = zbus::fdo::DBusProxy::new(client).await.expect("dbus");
    let bus_name = zbus::names::BusName::try_from(name).expect("bus name");
    // Subscribed before the first look, so a change between the two is not lost.
    let mut changes = dbus
        .receive_name_owner_changed_with_args(&[(0, name)])
        .await
        .expect("subscribe to the name's owner");
    while dbus.name_has_owner(bus_name.clone()).await.expect("ask") != owned {
        tokio::time::timeout(base_testkit::GENEROUS, changes.next())
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "{name} to {}",
                    if owned {
                        "get an owner"
                    } else {
                        "lose its owner"
                    }
                )
            });
    }
}
