//! A JMAP server a person typed by hand, sent: the session resource at the URL they gave,
//! fetched with HTTP Basic (the login name and the password) or, for an API token, a bearer,
//! read the way discovery reads a session it found. The request and the reading of the answer
//! are account-generic-proto's.

use super::fault;
use account_capability::Claim;
use account_core::{Credential, EndpointUrl, LoginName, ServiceEndpoint};
use account_discover::{DiscoverError, parse_jmap_session};
use account_family_kit::Io;
use account_family_kit::password::{basic, fault_of, sensitive};
use account_generic_proto::jmap;
use account_sheet::SignInFault;
use http_core::Http;

/// The JMAP endpoint and what the session offers, logged in as `login`.
pub(super) async fn session(
    io: &Io,
    url: &EndpointUrl,
    login: &LoginName,
    credential: &Credential,
) -> Result<(Vec<ServiceEndpoint>, Vec<Claim>), SignInFault> {
    let authorization = match credential {
        Credential::Password(password) => basic(&login.0, password),
        Credential::Bearer(token) => format!("Bearer {}", token.expose()),
        _ => return Err(SignInFault::Unreadable),
    };
    let request =
        jmap::request(url, sensitive(&authorization).map_err(fault_of)?).map_err(fault)?;
    let response = io.http.send(request).await.map_err(fault_of)?;
    let body = jmap::session_text(&response).map_err(fault)?;
    let found = parse_jmap_session(body).map_err(discover_fault)?;
    Ok((jmap::as_login(found.endpoints, login), found.claims))
}

/// What a session document the discovery step could not use means for a sign-in.
#[expect(
    clippy::needless_pass_by_value,
    reason = "passed by name to `map_err`, which hands the error over by value"
)]
fn discover_fault(fault: DiscoverError) -> SignInFault {
    match fault {
        DiscoverError::Unreachable | DiscoverError::Transport(_) => SignInFault::Unreachable,
        DiscoverError::NoServers
        | DiscoverError::Unreadable
        | DiscoverError::Unparsed(_)
        | DiscoverError::BadEndpoint(_) => SignInFault::Unreadable,
    }
}
