//! Taking an app password back, sent. The request and the reading of the answer are
//! account-nextcloud-proto's.

use account_core::{EndpointUrl, LoginName, SecretText};
use account_family_kit::Io;
use account_family_kit::password::{basic, sensitive};
use account_http::{Http, HttpError};
use account_nextcloud_proto::Error;
use account_nextcloud_proto::revoke::{self, Revoked};
use account_provider::{ProviderError, RevokeOutcome};

/// Deletes the app password. A server that already refuses it has nothing left to revoke; one
/// without the endpoint (before Nextcloud 13) sends the person to their security settings.
pub(super) async fn revoke(
    io: &Io,
    server: &EndpointUrl,
    login: &LoginName,
    password: &SecretText,
) -> Result<RevokeOutcome, ProviderError> {
    let authorization = sensitive(&basic(&login.0, password)).map_err(http_fault)?;
    let request = revoke::request(server, authorization).map_err(proto_fault)?;
    let response = io.http.send(request).await.map_err(http_fault)?;
    match revoke::revoked(server, &response) {
        Ok(Revoked::Done) => Ok(RevokeOutcome::Revoked),
        Ok(Revoked::Manual(page)) => Ok(RevokeOutcome::Manual(page)),
        Err(error) => Err(proto_fault(error)),
    }
}

/// What a request that went nowhere says about the account.
fn http_fault(error: HttpError) -> ProviderError {
    match error {
        HttpError::Unreachable(..)
        | HttpError::Tls(..)
        | HttpError::TimedOut
        | HttpError::NotGranted => ProviderError::Unreachable,
        // too large, malformed, and a failure a later account-http adds
        _ => ProviderError::Unreadable,
    }
}

/// What a request that could not be built or an answer that could not be read says about it.
fn proto_fault(error: Error) -> ProviderError {
    match error {
        Error::Unreachable => ProviderError::Unreachable,
        _ => ProviderError::Unreadable,
    }
}
