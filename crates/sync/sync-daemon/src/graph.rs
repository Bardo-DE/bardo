//! The Graph replica as syncd runs it.
//!
//! The same relays as the `WebDAV` replica ([`RelayDial`]: one authenticated stream per connection,
//! from `OpenAuthenticated` on the account service syncd hosts itself), so the replica never holds
//! the Microsoft token. The relay adds `Authorization: Bearer` and reaches the endpoint's origin
//! (`https://graph.microsoft.com`). Each new connection asks the service for a token, which
//! refreshes the account's sign-in; a refresh the issuer refuses leaves the account `NeedsReauth`
//! and the dial fails. A connection kept idle holds the bearer it was opened with: when that has
//! lapsed the server answers `401`, and `RelayHttp` drops the kept connections and sends the
//! request again on a new one, which carries a fresh bearer.
//!
//! Graph's pre-authenticated links (an upload session's `uploadUrl`, the `downloadUrl` a content
//! request redirects to) are on other hosts. Requests to them go through [`RelayDial::linked`]:
//! `OpenLinked`, a relay that adds no credential and that the service opens only for an origin
//! the provider file declares (`linked_origins`). [`graph_replica`] builds the replica of a
//! dataset's folder in the account's `OneDrive` app folder over both. Which grant and endpoint a
//! Storage account on a Microsoft account is reached through is the service's candidate: the
//! `graph` endpoint under a Storage grant.

use crate::webdav::BuildError;
use account_client::{Grants, GrantsLink};
use account_core::EndpointUrl;
use account_core::GrantId;
use http_relay::{RelayDial, RelayHttp, RelayLimits, Routed, bare_url};
use base_url::{Origin, WebUrl};
use replica_graph::GraphReplica;
use std::sync::Arc;
use sync_core::Clock;

/// Makes the client for one linked origin.
type MakeLinked<T> = Box<dyn Fn(&Origin) -> RelayHttp<RelayDial<T>> + Send + Sync>;

/// What the replica sends through: the Graph host over the authenticated relay, any other origin
/// over a linked one.
pub type GraphHttp<T> = Routed<RelayHttp<RelayDial<T>>, RelayHttp<RelayDial<T>>, MakeLinked<T>>;

/// The replica of `folder` of the account the grant covers.
///
/// `folder` is a path in the app folder, `Photos/Originals`; empty for the app folder itself.
/// It runs over relays the account service opens to
/// `endpoint` and, for the links Graph hands out, to the origins of those links.
///
/// # Errors
///
/// Returns [`BuildError::NotWeb`] when `endpoint` is not a web address, or another [`BuildError`]
/// when the client or `folder` cannot be used.
pub fn graph_replica<T: GrantsLink + 'static>(
    accounts: Arc<Grants<T>>,
    grant: GrantId,
    endpoint: EndpointUrl,
    folder: &str,
    clock: Clock,
) -> Result<GraphReplica<GraphHttp<T>>, BuildError> {
    let base = WebUrl::try_from(&endpoint).map_err(BuildError::NotWeb)?;
    let home = RelayHttp::new(
        RelayDial::authenticated(Arc::clone(&accounts), grant.clone(), endpoint),
        RelayLimits::default(),
    );
    let linked: MakeLinked<T> = Box::new(move |origin| {
        RelayHttp::new(
            RelayDial::linked(Arc::clone(&accounts), grant.clone(), bare_url(origin)),
            RelayLimits::default(),
        )
    });
    Ok(GraphReplica::new(
        Routed::new(home, &base, linked),
        &base,
        folder,
        clock,
    ))
}
