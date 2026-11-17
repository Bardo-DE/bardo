//! The mail search: every source in order, stopping at the first usable answer.
//!
//! Ported from `search` and `fetch` in mailo's `crates/mail-runtime/src/discover.rs` (MIT OR
//! Apache-2.0, same author), over the [`Http`] and [`Dns`] seams instead of reqwest and
//! hickory. The order:
//!
//! 1. a provider porter knows that lists the address's domain (no network at all);
//! 2. the domain's own autoconfig document, its `.well-known` path, then the ISPDB;
//! 3. RFC 6186 SRV records;
//! 4. the MX records: a provider that claims an MX host, or the ISPDB document of the host's
//!    operator.
//!
//! HTTPS only, and no plain-HTTP fallback: `EndpointUrl`s here are all `https`, and a document
//! fetched in the clear could name any server at all. The `Http` implementation owns the
//! per-request timeout, size cap and redirect policy; this caps the document it will read.
//! Nothing is sent to a discovered server: the result goes to the review step.

use crate::autoconfig::{autoconfig_urls, domain_of, ispdb_url, parse_autoconfig_with};
use crate::dns::{Dns, DnsFault};
use crate::found::{Cause, DiscoverFault, Found, SearchOptions, Source};
use crate::mx::{ProviderLead, ispdb_candidates, lookup_mx, provider_leads};
use crate::srv::{found_from_srv_with, lookup_srv};
use account_core::Tls;
use http_core::http::Method;
use http_core::{Http, HttpError, request};
use account_provider::{DomainMatch, ProviderSet};
use base_url::WebUrl;
use std::fmt;
use std::sync::Arc;

/// The most a configuration document may weigh. Real ones are a few kilobytes.
const MAX_DOCUMENT: usize = 256 * 1024;

/// What the search came to: servers, or a provider porter already describes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Servers a source named.
    Servers(Found),
    /// A provider file claims the address; its own discovery takes over.
    Provider(ProviderLead),
}

/// Why one step found nothing.
///
/// Two misses are equal when they are the same kind of failure: [`Self::Down`] is the kind
/// [`Self::Unreachable`] is, and [`Self::Garbled`] the kind [`Self::Malformed`] is; the cause does
/// not count.
#[derive(Debug, Clone)]
pub enum Miss {
    /// The source answered: there is nothing there (404, no records).
    Absent,
    /// No answer: DNS, the connection or TLS failed, or it timed out.
    Unreachable,
    /// An answer that is not a configuration: another status, or a document that did not parse.
    Malformed,
    /// A configuration that cannot be used, and why.
    Unusable(DiscoverFault),
    /// No answer, and why.
    Down(Cause),
    /// An answer that is not a configuration, and why.
    Garbled(Cause),
}

impl Miss {
    fn down(cause: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Down(Arc::from(cause.into()))
    }

    fn garbled(cause: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Garbled(Arc::from(cause.into()))
    }

    fn kind(&self) -> u8 {
        match self {
            Self::Absent => 0,
            Self::Unreachable | Self::Down(_) => 1,
            Self::Malformed | Self::Garbled(_) => 2,
            Self::Unusable(_) => 3,
        }
    }
}

impl PartialEq for Miss {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unusable(a), Self::Unusable(b)) => a == b,
            _ => self.kind() == other.kind(),
        }
    }
}

impl Eq for Miss {}

/// One step of the search and what came of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tried {
    /// The URL fetched or the DNS name asked about.
    pub what: String,
    /// What came of it.
    pub miss: Miss,
}

/// The search found nothing usable. Every step is listed, so the user sees what was asked.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub struct NotFound {
    /// Every step, in order.
    pub tried: Vec<Tried>,
}

impl NotFound {
    /// Nothing answered at all: most likely this computer is offline.
    #[must_use]
    pub fn offline(&self) -> bool {
        !self.tried.is_empty() && self.tried.iter().all(|t| t.miss == Miss::Unreachable)
    }
}

impl fmt::Display for NotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.offline() {
            f.write_str("could not reach anything to ask; is this computer offline?")?;
        } else {
            f.write_str("no usable configuration was found")?;
        }
        self.tried.iter().try_for_each(|tried| {
            let why = match tried.miss {
                Miss::Absent => "nothing there",
                Miss::Unreachable | Miss::Down(_) => "no answer",
                Miss::Malformed | Miss::Garbled(_) => "not a configuration",
                Miss::Unusable(DiscoverFault::NoServers) => "names no usable server",
                Miss::Unusable(_) => "unusable",
            };
            write!(f, "\n  {}: {why}", tried.what)
        })
    }
}

/// Searches every source in order for `address`, stopping at the first usable answer, under the
/// default [`SearchOptions`] (`STARTTLS` accepted, POP3 ignored).
///
/// # Errors
///
/// `NotFound` when no source names a usable answer.
pub async fn discover_mail<H: Http, D: Dns>(
    http: &H,
    dns: &D,
    providers: &ProviderSet,
    address: &str,
) -> Result<Outcome, NotFound> {
    discover_mail_with(http, dns, providers, address, &SearchOptions::default()).await
}

/// [`discover_mail`] under `options`.
///
/// With `starttls_only`, a source whose servers need `STARTTLS` (a document, or SRV submission) is
/// a miss and the next source is tried, as mailo did; with `report_pop3`, documents' POP3 servers
/// come back on [`Found::pop3`] and a POP3-only document is a finding.
///
/// # Errors
///
/// `NotFound` when no source names a usable answer under `options`.
pub async fn discover_mail_with<H: Http, D: Dns>(
    http: &H,
    dns: &D,
    providers: &ProviderSet,
    address: &str,
    options: &SearchOptions,
) -> Result<Outcome, NotFound> {
    let mut tried = Vec::new();
    match search(http, dns, providers, address, options, &mut tried).await {
        Some(outcome) => Ok(outcome),
        None => Err(NotFound { tried }),
    }
}

async fn search<H: Http, D: Dns>(
    http: &H,
    dns: &D,
    providers: &ProviderSet,
    address: &str,
    options: &SearchOptions,
    tried: &mut Vec<Tried>,
) -> Option<Outcome> {
    let Some(domain) = domain_of(address) else {
        tried.push(Tried {
            what: address.to_owned(),
            miss: Miss::Malformed,
        });
        return None;
    };

    // 1. A provider that lists the domain.
    if let Some(lead) = provider_leads(providers, &domain, &[]).into_iter().next() {
        return Some(Outcome::Provider(lead));
    }

    // 2. Documents.
    for url in autoconfig_urls(&domain, address) {
        if let Some(found) = document(http, &url, address, Source::Autoconfig, options, tried).await
        {
            return Some(Outcome::Servers(found));
        }
    }

    // 3. SRV.
    match lookup_srv(dns, &domain).await {
        Ok(answers) => match found_from_srv_with(address, &answers, options.report_pop3)
            .and_then(|f| tls_ok(f, *options))
        {
            Ok(found) => return Some(Outcome::Servers(found)),
            Err(why) => tried.push(Tried {
                what: format!("SRV records for {domain}"),
                miss: match why {
                    DiscoverFault::NoServers => Miss::Absent,
                    other => Miss::Unusable(other),
                },
            }),
        },
        Err(fault) => tried.push(Tried {
            what: format!("SRV records for {domain}"),
            miss: dns_miss(fault),
        }),
    }

    // 4. MX.
    let records = match lookup_mx(dns, &domain).await {
        Ok(records) => records,
        Err(fault) => {
            tried.push(Tried {
                what: format!("MX {domain}"),
                miss: dns_miss(fault),
            });
            return None;
        }
    };
    if let Some(lead) = provider_leads(providers, &domain, &records)
        .into_iter()
        .find(|lead| lead.via == DomainMatch::Mx)
    {
        return Some(Outcome::Provider(lead));
    }
    let host = crate::mx::mx_hosts(&records).into_iter().next();
    let candidates = host
        .map(|host| ispdb_candidates(&host, &domain))
        .unwrap_or_default();
    for candidate in candidates {
        let Ok(url) = WebUrl::parse(&ispdb_url(&candidate)) else {
            continue;
        };
        if let Some(found) = document(http, &url, address, Source::Mx, options, tried).await {
            return Some(Outcome::Servers(found));
        }
    }
    tried.push(Tried {
        what: format!("MX {domain}"),
        miss: Miss::Absent,
    });
    None
}

/// Under `starttls_only`, a finding with a `STARTTLS` endpoint is not one.
fn tls_ok(found: Found, options: SearchOptions) -> Result<Found, DiscoverFault> {
    let starttls = found.endpoints.iter().any(|e| e.tls == Tls::StartTls);
    match (options.starttls_only, starttls) {
        (true, true) => Err(DiscoverFault::NoServers),
        _ => Ok(found),
    }
}

fn dns_miss(fault: DnsFault) -> Miss {
    match fault {
        DnsFault::NoRecords => Miss::Absent,
        DnsFault::Unreachable => Miss::Unreachable,
        DnsFault::Resolver(cause) => Miss::Down(cause),
    }
}

/// Fetches one document and reads it, recording why not when it cannot be used. `source`
/// replaces `Autoconfig` when the document was reached through an MX lead.
async fn document<H: Http>(
    http: &H,
    url: &WebUrl,
    address: &str,
    source: Source,
    options: &SearchOptions,
    tried: &mut Vec<Tried>,
) -> Option<Found> {
    let miss = match fetch(http, url).await {
        Ok(body) => match parse_autoconfig_with(&body, address, options) {
            Ok(found) => return Some(Found { source, ..found }),
            Err(DiscoverFault::Unreadable) => Miss::Malformed,
            Err(DiscoverFault::Unparsed(cause)) => Miss::Garbled(cause),
            Err(why) => Miss::Unusable(why),
        },
        Err(miss) => miss,
    };
    tried.push(Tried {
        what: url.to_string(),
        miss,
    });
    None
}

/// GETs a document, at most [`MAX_DOCUMENT`] of it.
async fn fetch<H: Http>(http: &H, url: &WebUrl) -> Result<String, Miss> {
    let asked = request(Method::GET, url)
        .body(Vec::new())
        .map_err(Miss::garbled)?;
    let response = http.send(asked).await.map_err(|error| match error {
        HttpError::TooLarge | HttpError::Malformed(..) => Miss::garbled(error),
        // unreachable, TLS, timed out, not granted, and a failure a later http-core adds
        _ => Miss::down(error),
    })?;
    match response.status().as_u16() {
        404 | 410 => Err(Miss::Absent),
        _ if !response.status().is_success() => Err(Miss::Malformed),
        _ if response.body().len() > MAX_DOCUMENT => Err(Miss::Malformed),
        _ => String::from_utf8(response.into_body()).map_err(Miss::garbled),
    }
}

#[cfg(test)]
mod tests;
