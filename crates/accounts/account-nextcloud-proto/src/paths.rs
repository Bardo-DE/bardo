//! Where a Nextcloud keeps things, under the server's root (which may itself be a sub-path).

use account_core::{EndpointUrl, LoginName, ServiceEndpoint};

/// Where a Nextcloud's own paths start, as a marker in an endpoint's URL.
const MARKERS: [&str; 2] = ["/remote.php/", "/index.php/"];

/// The server root and the login of the first endpoint that sits under a Nextcloud's own paths
/// (every endpoint of a Nextcloud account is under the server's root).
#[must_use]
pub fn server_of(endpoints: &[ServiceEndpoint]) -> Option<(EndpointUrl, LoginName)> {
    endpoints.iter().find_map(|endpoint| {
        let text = endpoint.url.as_str();
        let root = MARKERS
            .iter()
            .filter_map(|marker| text.find(marker))
            .min()
            .map(|at| &text[..at])?;
        let server = EndpointUrl::parse(root).ok()?;
        Some((server, endpoint.login.clone()))
    })
}

/// The root of the DAV tree.
#[must_use]
pub fn dav_root(server: &EndpointUrl) -> Option<EndpointUrl> {
    server.join("/remote.php/dav/")
}

/// The OCS capabilities.
#[must_use]
pub fn capabilities(server: &EndpointUrl) -> Option<EndpointUrl> {
    server.join("/ocs/v2.php/cloud/capabilities")
}

/// Where Login Flow v2 starts.
#[must_use]
pub fn login_flow(server: &EndpointUrl) -> Option<EndpointUrl> {
    server.join("/index.php/login/v2")
}

/// The app password, which `DELETE` takes back.
#[must_use]
pub fn app_password(server: &EndpointUrl) -> Option<EndpointUrl> {
    server.join("/ocs/v2.php/core/apppassword")
}

/// The page where a person takes an app password back by hand.
#[must_use]
pub fn security_settings(server: &EndpointUrl) -> Option<EndpointUrl> {
    server.join("/settings/user/security")
}

/// The Notes API.
#[must_use]
pub fn notes(server: &EndpointUrl) -> Option<EndpointUrl> {
    server.join("/index.php/apps/notes/api/v1/")
}

/// A user's files root.
#[must_use]
pub fn files(server: &EndpointUrl, user: &str) -> Option<EndpointUrl> {
    server.join(&format!("/remote.php/dav/files/{}/", segment(user)))
}

/// Where a user's calendars are by convention, for a server that names no home.
#[must_use]
pub fn calendars(server: &EndpointUrl, user: &str) -> Option<EndpointUrl> {
    server.join(&format!("/remote.php/dav/calendars/{}/", segment(user)))
}

/// Where a user's address books are by convention, for a server that names no home.
#[must_use]
pub fn address_books(server: &EndpointUrl, user: &str) -> Option<EndpointUrl> {
    server.join(&format!(
        "/remote.php/dav/addressbooks/users/{}/",
        segment(user)
    ))
}

/// The user id at the end of a principal path (`/remote.php/dav/principals/users/ada/`),
/// decoded.
#[must_use]
pub fn user_of(path: &str) -> Option<String> {
    let last = path.trim_end_matches('/').rsplit('/').next()?;
    let decoded = base_url::percent::decode_strict(last)?;
    (!decoded.is_empty()).then_some(decoded)
}

/// A path segment, percent-encoded (an `@` or a space in a user id is not a URL character).
fn segment(text: &str) -> String {
    base_url::percent::encode_unreserved(text)
}

#[cfg(test)]
mod tests;
