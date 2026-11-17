//! The builders of the families that live in their own crates: each serves a spec when its
//! sign-in (the `AuthKind`, the discovery, the issuer) is the family's own, and hands it back
//! otherwise.

use super::{Built, FamilyIo};
use crate::dispatch::DynProvider;
use account_core::AuthKind;
use account_family_generic::{DavLookup, FixedLookup, GenericProvider, Lookup, MailLookup};
use account_family_nextcloud::NextcloudProvider;
use account_family_oauth::{BrowserIssuer, Google, Microsoft, OAuthEnv, OAuthProvider, Standard};
use http_reqwest::ReqwestHttp;
use account_provider::{Discovery, Issuer, ProviderSpec};

/// Nextcloud, when `spec` signs in by Login Flow v2.
pub(super) fn nextcloud(spec: ProviderSpec, io: &FamilyIo) -> Built {
    match spec.auth.kind {
        AuthKind::LoginFlowV2 => {
            let sleep = http_reqwest::TokioSleep;
            Ok(Box::new(NextcloudProvider::new(
                spec,
                io.http.clone(),
                sleep,
            )))
        }
        _ => Err(Box::new(spec)),
    }
}

/// The generic family, when `spec` signs in by a password or an app password. How the servers
/// are found is the lookup its discovery names: an address (autoconfig, or a JMAP server the
/// person types), a server's name (`.well-known`), or the file's own servers.
pub(super) fn generic(spec: ProviderSpec, io: &FamilyIo) -> Built {
    fn serving<L: Lookup>(spec: ProviderSpec, io: &FamilyIo) -> Box<dyn DynProvider> {
        let provider = GenericProvider::<L>::new(spec, io.http.clone(), io.dns.clone());
        Box::new(provider.with_providers(io.providers.clone()))
    }
    match (spec.auth.kind, &spec.discovery) {
        (
            AuthKind::Password | AuthKind::AppPassword,
            Discovery::Autoconfig | Discovery::JmapSession,
        ) => Ok(serving::<MailLookup>(spec, io)),
        (AuthKind::Password | AuthKind::AppPassword, Discovery::WellKnown) => {
            Ok(serving::<DavLookup>(spec, io))
        }
        (AuthKind::Password | AuthKind::AppPassword, Discovery::Fixed) => {
            Ok(serving::<FixedLookup>(spec, io))
        }
        _ => Err(Box::new(spec)),
    }
}

/// OAuth, when `spec` signs in by PKCE at an issuer it has rules for (Google, Microsoft) or one
/// whose data says who answered ([`Standard`]).
pub(super) fn oauth(spec: ProviderSpec, io: &FamilyIo) -> Built {
    fn serving<R: BrowserIssuer>(
        rules: R,
        spec: ProviderSpec,
        io: &FamilyIo,
    ) -> Box<dyn DynProvider> {
        let clock = std::sync::Arc::clone(&io.clock);
        let env = OAuthEnv::<ReqwestHttp>::with_client_files(io.clients.clone(), clock);
        Box::new(OAuthProvider::with_env(rules, spec, env))
    }
    match (spec.auth.kind, spec.auth.issuer) {
        (AuthKind::OAuthPkce, Some(Issuer::Google)) => Ok(serving(Google, spec, io)),
        (AuthKind::OAuthPkce, Some(Issuer::Microsoft)) => Ok(serving(Microsoft, spec, io)),
        (AuthKind::OAuthPkce, Some(issuer)) if Standard::serves(issuer) => {
            Ok(serving(Standard::of(issuer), spec, io))
        }
        _ => Err(Box::new(spec)),
    }
}
