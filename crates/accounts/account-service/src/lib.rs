//! accountd's core (design/31 §4.1): the account registry and consent over its seams, as the
//! facade [`AccountService`] over the domains that do the work: `account-service-accounts` (add,
//! sign in again, remove, rediscover), `account-service-grants` (who may use which account, the
//! relays a grant opens) and what they share, `account-service-core` (the registry, the audit
//! trail, the sheets seam, the reach policy, the context). The seams are
//! [`account_provider::Provider`] (protocol code), [`account_secrets::Secrets`], `Sheets` (the host
//! that draws consent and sign-in), a `Clock`, a `RegistryStore` (where the registry lives
//! between runs) and an `AuditSink`. Transport-free: accountd hosts it behind D-Bus, and an app
//! may host it in process.
//!
//! An OAuth account is refreshed one caller at a time inside a service: a second caller waits and
//! then uses the token the first one issued (one call to the issuer serves both), and a refresh
//! token the issuer rotated is stored before the new access token is handed out. A host that
//! keeps its registry on disk passes `account_store::FileStore` to
//! [`AccountService::with_store`]; this crate depends on no executor, so the file store lives in
//! `account-store`.

mod service;

pub use service::{AccountService, Service};
