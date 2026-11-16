//! The mail servers of a generic account that need no search: the ones a person typed, and the
//! ones a provider file names.

use crate::{Error, declared};
use account_capability::{Capability, MailCap, MailTransport};
use account_capability::{Claim, Offer};
use account_core::{EndpointUrl, Family, LoginName, ServiceEndpoint, Tls};
use account_provider::{DomainName, ProviderSpec};
use account_sheet::{Hop, MailServers};
use base_url::UrlScheme;

/// The domain of an address, when it is one.
#[must_use]
pub fn domain_of(address: &str) -> Option<DomainName> {
    let (local, domain) = address.rsplit_once('@')?;
    (!local.is_empty()).then(|| DomainName::parse(domain).ok())?
}

/// One typed server as an endpoint: the scheme says the family and, with the security, whether
/// the connection is encrypted from the first byte (`imaps`, `pop3s`, `smtps`), upgraded (`imap`,
/// `pop3`, `smtp` with STARTTLS) or plain (this computer only; `check` refuses it anywhere else).
fn endpoint(family: Family, hop: &Hop, login: &LoginName) -> Option<ServiceEndpoint> {
    let (scheme, tls) = match (family, hop.security) {
        (Family::Imap, Tls::Implicit) => ("imaps", Tls::Implicit),
        (Family::Imap, Tls::StartTls) => ("imap", Tls::StartTls),
        (Family::Imap, Tls::Plain) => ("imap", Tls::Plain),
        (Family::Pop3, Tls::Implicit) => ("pop3s", Tls::Implicit),
        (Family::Pop3, Tls::StartTls) => ("pop3", Tls::StartTls),
        (Family::Pop3, Tls::Plain) => ("pop3", Tls::Plain),
        (Family::Smtp, Tls::Implicit) => ("smtps", Tls::Implicit),
        (Family::Smtp, Tls::StartTls) => ("smtp", Tls::StartTls),
        (Family::Smtp, Tls::Plain) => ("smtp", Tls::Plain),
        _ => return None,
    };
    let endpoint = ServiceEndpoint {
        family,
        url: EndpointUrl::parse(&format!("{scheme}://{}:{}", hop.host, hop.port)).ok()?,
        tls,
        login: login.clone(),
    };
    endpoint.check().ok().map(|()| endpoint)
}

/// The servers a person typed for an IMAP or POP3 account (`incoming`).
///
/// That protocol and SMTP at the hosts, ports and securities given, both logged in as the typed
/// login name, or as the address when there is none.
///
/// # Errors
///
/// [`Error::Unreadable`] when a typed host, port or security does not make a server address.
pub fn typed(
    incoming: Family,
    servers: &MailServers,
    address: &str,
    spec: &ProviderSpec,
) -> Result<(Vec<ServiceEndpoint>, Vec<Claim>), Error> {
    let login = LoginName(servers.login.clone().unwrap_or_else(|| address.to_owned()));
    let endpoints = [
        endpoint(incoming, &servers.incoming, &login),
        endpoint(Family::Smtp, &servers.outgoing, &login),
    ]
    .into_iter()
    .collect::<Option<Vec<_>>>()
    .ok_or(Error::Unreadable)?;
    Ok((endpoints, claims_for(incoming, declared(spec))))
}

/// The file's claims for an account read by `incoming`: the file describes IMAP, so a POP3
/// account's mail claim names POP3, which an app that plans around the transport sees.
fn claims_for(incoming: Family, claims: Vec<Claim>) -> Vec<Claim> {
    if incoming != Family::Pop3 {
        return claims;
    }
    claims
        .into_iter()
        .map(|claim| match claim.offer {
            Offer::Present(Capability::Mail(mail)) => Claim {
                offer: Offer::Present(Capability::Mail(MailCap {
                    transport: MailTransport::Pop3,
                    ..mail
                })),
                ..claim
            },
            _ => claim,
        })
        .collect()
}

/// The TLS a file's endpoint URL means: `imaps`, `smtps` and `https` are implicit TLS; `imap`,
/// `smtp`, `pop3` and `sieve` upgrade with STARTTLS, except on this computer, where they may stay plain
/// (a fake server in a test). Anything else has no mail meaning.
fn tls_of_scheme(url: &EndpointUrl) -> Option<Tls> {
    let origin = url.origin();
    match origin.scheme {
        UrlScheme::Imaps
        | UrlScheme::Smtps
        | UrlScheme::Pop3s
        | UrlScheme::Https
        | UrlScheme::Sieves => Some(Tls::Implicit),
        UrlScheme::Imap
        | UrlScheme::Smtp
        | UrlScheme::Pop3
        | UrlScheme::Sieve
        | UrlScheme::Http
            if origin.is_loopback() =>
        {
            Some(Tls::Plain)
        }
        UrlScheme::Imap | UrlScheme::Smtp | UrlScheme::Pop3 | UrlScheme::Sieve => {
            Some(Tls::StartTls)
        }
        UrlScheme::Http => None,
    }
}

/// The servers a provider file with `discovery = "fixed"` names.
///
/// One endpoint per capability row that has a URL, its family and TLS from the row and its scheme,
/// logged in as the address. The claims are the file's, once each (an SMTP row repeats its IMAP
/// row's mail capability).
///
/// # Errors
///
/// [`Error::Unreadable`] when a row's URL does not parse or its scheme names no TLS mode.
pub fn fixed(
    address: &str,
    spec: &ProviderSpec,
) -> Result<(Vec<ServiceEndpoint>, Vec<Claim>), Error> {
    let login = LoginName(address.to_owned());
    let endpoints = spec
        .capabilities
        .iter()
        .filter_map(|row| row.endpoint.as_ref().map(|url| (row.family, url)))
        .map(|(family, url)| {
            let url = EndpointUrl::parse(url).ok().ok_or(Error::Unreadable)?;
            let endpoint = ServiceEndpoint {
                family,
                tls: tls_of_scheme(&url).ok_or(Error::Unreadable)?,
                url,
                login: login.clone(),
            };
            endpoint
                .check()
                .map(|()| endpoint)
                .ok()
                .ok_or(Error::Unreadable)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if endpoints.is_empty() {
        return Err(Error::Unreadable);
    }
    let mut claims: Vec<Claim> = Vec::new();
    for claim in declared(spec) {
        if !claims.contains(&claim) {
            claims.push(claim);
        }
    }
    Ok((endpoints, claims))
}

#[cfg(test)]
mod tests;
