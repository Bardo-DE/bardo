//! What the family tests share: an HTTP client that sends only to the fakes, and a person who
//! answers sheets.
#![allow(
    dead_code,
    reason = "each test file uses a part of the shared helpers, so some are unused in each"
)]

#[cfg(feature = "generic")]
pub mod mail;

use account_capability::KindToggle;
use account_core::SecretText;
use http_core::http::{Request, Response};
use http_core::{Http, HttpError};
use http_reqwest::ReqwestHttp;
use account_provider::{SignIn, SignInStep};
use account_sheet::{FieldAnswer, FieldKind, FieldValue, SheetInput, SheetView};
use std::sync::{Arc, Mutex};

/// One place a name is served: `host`'s paths under `prefix` go to `port` on loopback.
#[derive(Debug, Clone)]
pub struct Place {
    pub host: &'static str,
    pub prefix: &'static str,
    pub port: u16,
}

/// Sends `https://<host>/...` to the loopback port the table names for it, over plain HTTP, and
/// sends anything else nowhere (a family test never reaches the network). A request to a
/// loopback address goes straight to it.
#[derive(Debug, Clone)]
pub struct Fakes {
    pub places: Vec<Place>,
    pub inner: ReqwestHttp,
    pub sent: Arc<Mutex<Vec<String>>>,
}

impl Fakes {
    pub fn new(places: Vec<Place>) -> Self {
        Self {
            places,
            inner: ReqwestHttp::new(),
            sent: Arc::default(),
        }
    }

    /// Only loopback addresses are reachable.
    pub fn loopback() -> Self {
        Self::new(Vec::new())
    }
}

impl Http for Fakes {
    async fn send(&self, mut request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, HttpError> {
        let url =
            base_url::WebUrl::parse(&request.uri().to_string()).map_err(HttpError::malformed)?;
        let origin = url.origin();
        self.sent
            .lock()
            .expect("sent")
            .push(format!("{} {}", request.method(), url));
        if !origin.is_loopback() {
            let place = self
                .places
                .iter()
                .filter(|p| p.host == origin.host && url.path().starts_with(p.prefix))
                .max_by_key(|p| p.prefix.len())
                .ok_or(HttpError::Unreachable(None))?;
            *request.uri_mut() = format!("http://127.0.0.1:{}{}", place.port, url.path())
                .parse()
                .map_err(HttpError::malformed)?;
        }
        self.inner.send(request).await
    }
}

/// Waits a few milliseconds whatever it is asked, so a poll loop does not spin while a person
/// takes their time. A pacing stand-in for a real wait, not a wait for something to happen.
#[derive(Debug, Clone, Copy)]
pub struct Brief;

impl http_core::Sleep for Brief {
    async fn sleep(&self, _how_long: std::time::Duration) {
        tokio::time::sleep(std::time::Duration::from_millis(3)).await;
    }
}

pub fn plain(kind: FieldKind, text: &str) -> FieldAnswer {
    FieldAnswer {
        kind,
        value: FieldValue::Plain(text.to_owned()),
    }
}

pub fn secret(kind: FieldKind, text: &str) -> FieldAnswer {
    FieldAnswer {
        kind,
        value: FieldValue::Secret(SecretText::new(text)),
    }
}

/// Every service of a review, on.
pub fn all_on(
    kinds: impl IntoIterator<Item = account_capability::CapabilityKind>,
) -> Vec<KindToggle> {
    kinds
        .into_iter()
        .map(|kind| KindToggle { kind, toggle: true })
        .collect()
}

/// Feeds a sign-in `Start`, then what `answer` says to each step, until it is done or failed;
/// every step in order.
pub async fn drive<S: SignIn>(
    signin: &mut S,
    mut answer: impl FnMut(&SignInStep) -> account_sheet::SignInInput,
) -> Vec<SignInStep> {
    use account_sheet::SignInInput;
    let mut steps = Vec::new();
    let mut input = SignInInput::Start;
    for _ in 0..500 {
        let step = signin.next(input).await;
        let over = matches!(step, SignInStep::Done(_) | SignInStep::Failed(_));
        input = answer(&step);
        steps.push(step);
        if over {
            return steps;
        }
    }
    panic!("the sign-in did not end: {steps:#?}");
}

// ---- a person at the sheets, and a service to hand them to ----

use account_capability::CapabilityKind;
use account_core::AccountId;
use account_core::consent::{ConsentAnswer, ConsentAsk};
use account_core::{Credential, SecretKey};
use account_fake::{MemoryStore, RecordingAudit, ScriptedSheets};
use account_families::DynProvider;
use account_secrets::{MemorySecrets, Secrets, SecretsError};
use account_service::AccountService;
use account_sheet::host::{SheetFault, SheetLink, SheetOpen, Sheets};
use account_wire::ParentWindow;
use base_time::FixedClock;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

/// When the person opens the page a browser step shows.
#[derive(Clone)]
pub enum Opens {
    /// As soon as it is shown.
    Now,
    /// Once this holds (a fake has seen what the person was waiting for).
    Once(std::sync::Arc<dyn Fn() -> bool + Send + Sync>),
}

impl Opens {
    /// Opens the page once `ready` holds.
    pub fn once(ready: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        Self::Once(std::sync::Arc::new(ready))
    }
}

impl std::fmt::Debug for Opens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Now => f.write_str("Now"),
            Self::Once(_) => f.write_str("Once(..)"),
        }
    }
}

/// What the person at the sheet does.
#[derive(Debug, Clone, Default)]
pub struct Person {
    /// The provider they pick from the list (the first when none).
    pub pick: Option<account_core::ProviderId>,
    /// What they type into any form.
    pub answers: Vec<FieldAnswer>,
    /// The services they turn off on the review.
    pub off: Vec<CapabilityKind>,
    /// Whether they open the page a browser step shows, and when.
    pub opens_page: Option<Opens>,
    /// Whether they close the sheet when it waits for the browser.
    pub gives_up: bool,
}

/// Sheets whose conversations are answered by `Person` (one per conversation, in order) and
/// whose consent asks go to a script.
#[derive(Debug)]
pub struct TestSheets {
    people: Mutex<std::collections::VecDeque<Person>>,
    consent: ScriptedSheets,
    shown: Arc<Mutex<Vec<SheetView>>>,
}

impl TestSheets {
    pub fn new(people: Vec<Person>, consent: ScriptedSheets) -> Self {
        Self {
            people: Mutex::new(people.into()),
            consent,
            shown: Arc::default(),
        }
    }

    pub fn shown(&self) -> Arc<Mutex<Vec<SheetView>>> {
        Arc::clone(&self.shown)
    }
}

#[derive(Debug)]
pub struct TestLink {
    person: Person,
    tx: UnboundedSender<SheetInput>,
    rx: UnboundedReceiver<SheetInput>,
    shown: Arc<Mutex<Vec<SheetView>>>,
}

impl TestLink {
    fn react(&self, view: &SheetView) {
        self.shown.lock().expect("shown").push(view.clone());
        let input = match view {
            SheetView::Providers(rows) => self
                .person
                .pick
                .clone()
                .or_else(|| rows.first().map(|r| r.id.clone()))
                .map(SheetInput::Pick),
            SheetView::SignIn(_) => Some(SheetInput::Submit(self.person.answers.clone())),
            SheetView::Review(review) => Some(SheetInput::Confirm(
                review
                    .review
                    .services
                    .iter()
                    .map(|row| KindToggle {
                        kind: row.kind,
                        toggle: !self.person.off.contains(&row.kind),
                    })
                    .collect(),
            )),
            SheetView::BrowserWait { url, .. } => {
                match (self.person.gives_up, self.person.opens_page.clone()) {
                    (true, _) => Some(SheetInput::Dismiss),
                    (false, Some(when)) => {
                        let url = url.to_string();
                        tokio::spawn(async move {
                            if let Opens::Once(ready) = when {
                                base_testkit::eventually("what the person waits for", || ready())
                                    .await;
                            }
                            open_page(&url).await;
                        });
                        None
                    }
                    (false, None) => None,
                }
            }
            // A sign-in that failed: the person reads it and closes the sheet.
            SheetView::Failed { .. } => Some(SheetInput::Dismiss),
            _ => None,
        };
        if let Some(input) = input {
            let _ = self.tx.send(input);
        }
    }
}

/// The person opens a page on a fake (a loopback address).
pub async fn open_page(url: &str) {
    use account_fake_servers::browser::split_loopback;
    use account_fake_servers::http::{Request, Scheme, send};
    let (address, target) = split_loopback(url).expect("a loopback page");
    send(&address, Scheme::Http, &Request::new("GET", &target))
        .await
        .expect("the page answers");
}

impl SheetLink for TestLink {
    #[expect(
        clippy::unused_async_trait_impl,
        reason = "`update` implements an async trait method, and this implementation has nothing to wait for"
    )]
    async fn update(&mut self, view: SheetView) -> Result<(), SheetFault> {
        self.react(&view);
        Ok(())
    }

    async fn input(&mut self) -> Result<SheetInput, SheetFault> {
        self.rx.recv().await.ok_or(SheetFault::Closed)
    }
}

impl Sheets for TestSheets {
    type Link = TestLink;

    async fn consent(&self, ask: ConsentAsk, window: &ParentWindow) -> ConsentAnswer {
        self.consent.consent(ask, window).await
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "`conversation` implements an async trait method, and this implementation has nothing to wait for"
    )]
    async fn conversation(&self, open: SheetOpen) -> Result<TestLink, SheetFault> {
        let person = self
            .people
            .lock()
            .expect("people")
            .pop_front()
            .ok_or(SheetFault::Unavailable)?;
        let (tx, rx) = unbounded_channel();
        let link = TestLink {
            person,
            tx,
            rx,
            shown: Arc::clone(&self.shown),
        };
        link.react(&open.view);
        Ok(link)
    }
}

/// A secret store the test keeps a handle on after the service has one.
#[derive(Debug, Clone, Default)]
pub struct SharedSecrets(pub Arc<MemorySecrets>);

impl SharedSecrets {
    pub async fn password(&self, account: &AccountId) -> Option<String> {
        let key = SecretKey {
            account: account.clone(),
            purpose: account_core::SecretPurpose::Password,
        };
        match self.0.get(&key).await {
            Ok(Credential::Password(p)) => Some(p.expose().to_owned()),
            _ => None,
        }
    }
}

impl Secrets for SharedSecrets {
    async fn put(&self, key: &SecretKey, value: &Credential) -> Result<(), SecretsError> {
        self.0.put(key, value).await
    }
    async fn get(&self, key: &SecretKey) -> Result<Credential, SecretsError> {
        self.0.get(key).await
    }
    async fn delete(&self, key: &SecretKey) -> Result<(), SecretsError> {
        self.0.delete(key).await
    }
    async fn delete_account(&self, account: &AccountId) -> Result<(), SecretsError> {
        self.0.delete_account(account).await
    }
}

pub type Service = AccountService<
    Box<dyn DynProvider>,
    SharedSecrets,
    TestSheets,
    FixedClock,
    MemoryStore,
    RecordingAudit,
>;

/// The service's seams, kept by the test.
pub struct Kept {
    pub secrets: SharedSecrets,
    pub store: MemoryStore,
    pub audit: RecordingAudit,
}

pub fn service(providers: Vec<Box<dyn DynProvider>>, sheets: TestSheets) -> (Service, Kept) {
    let kept = Kept {
        secrets: SharedSecrets::default(),
        store: MemoryStore::default(),
        audit: RecordingAudit::default(),
    };
    let service = AccountService::new(
        providers,
        account_service_core::registry::Registry::default(),
        kept.secrets.clone(),
        sheets,
        FixedClock(account_fake::NOW),
    )
    .with_store(kept.store.clone())
    .with_audit(kept.audit.clone());
    (service, kept)
}
