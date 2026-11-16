//! What the two password families share.
//!
//! These are Nextcloud's app password and a generic server's password. They share the form fields,
//! reading what was typed, server addresses, HTTP Basic, and the session a password account opens.
//!
//! A password never leaves accountd, so a session of these families mints no token: an app
//! reaches its servers through an authenticated relay (`OpenAuthenticated`), which reads the
//! credential itself.

use account_core::{Credential, SecretText};
use account_http::HttpError;
use account_http::http::Request;
use account_http::http::header::{AUTHORIZATION, HeaderValue};
use account_provider::{Presented, ProviderError, ProviderSpec};
use account_sheet::{Entry, FieldAnswer, FieldKind, FieldSpec, FieldValue, SignInFault};
use base64::Engine;

/// A form field.
#[must_use]
pub fn field(kind: FieldKind, entry: Entry, required: bool) -> FieldSpec {
    FieldSpec {
        kind,
        entry,
        required,
        prefill: None,
    }
}

/// The required, shown field of this kind.
#[must_use]
pub fn plain(kind: FieldKind) -> FieldSpec {
    field(kind, Entry::Plain, true)
}

/// The required, hidden password field.
#[must_use]
pub fn password() -> FieldSpec {
    field(FieldKind::Password, Entry::Secret, true)
}

/// The password field a provider asks for: an app password when its sign-in is `app_password`
/// (iCloud, Fastmail, Yahoo: the service takes no account password from a mail app), else a
/// password.
#[must_use]
pub fn password_for(spec: &ProviderSpec) -> FieldSpec {
    match spec.auth.kind {
        account_core::AuthKind::AppPassword => field(FieldKind::AppPassword, Entry::Secret, true),
        _ => password(),
    }
}

/// The password typed, in either field: an app password, else a password (a host may send
/// either kind for the field it was shown).
#[must_use]
pub fn typed_password(answers: &[FieldAnswer]) -> Option<SecretText> {
    secret_of(answers, FieldKind::AppPassword).or_else(|| secret_of(answers, FieldKind::Password))
}

/// What a request that went nowhere says about the sign-in.
#[must_use]
#[expect(
    clippy::needless_pass_by_value,
    reason = "passed by name to `map_err`, which hands the error over by value"
)]
pub fn fault_of(error: HttpError) -> SignInFault {
    match error {
        HttpError::Unreachable(..)
        | HttpError::Tls(..)
        | HttpError::TimedOut
        | HttpError::NotGranted => SignInFault::Unreachable,
        // too large, malformed, and a failure a later account-http adds
        _ => SignInFault::Unreadable,
    }
}

/// The plain text typed for `kind`, trimmed, when it is not empty.
#[must_use]
pub fn text_of(answers: &[FieldAnswer], kind: FieldKind) -> Option<String> {
    answers
        .iter()
        .find(|a| a.kind == kind)
        .map(|a| match &a.value {
            FieldValue::Plain(text) => text.trim().to_owned(),
            FieldValue::Secret(secret) => secret.expose().trim().to_owned(),
        })
        .filter(|text| !text.is_empty())
}

/// The secret typed for `kind`, exactly as typed (a password may start or end with a space).
#[must_use]
pub fn secret_of(answers: &[FieldAnswer], kind: FieldKind) -> Option<SecretText> {
    answers
        .iter()
        .find(|a| a.kind == kind)
        .map(|a| match &a.value {
            FieldValue::Secret(secret) => secret.clone(),
            FieldValue::Plain(text) => SecretText::new(text.clone()),
        })
        .filter(|secret| !secret.expose().is_empty())
}

/// The `Authorization` value of HTTP Basic.
#[must_use]
pub fn basic(login: &str, password: &SecretText) -> String {
    let token =
        base64::engine::general_purpose::STANDARD.encode(format!("{login}:{}", password.expose()));
    format!("Basic {token}")
}

/// `value` as a header value marked sensitive, so a request's `Debug` never shows it. A value
/// that cannot be a header's is `Malformed`.
///
/// # Errors
/// [`HttpError::Malformed`] when `value` is not a valid header value.
pub fn sensitive(value: &str) -> Result<HeaderValue, HttpError> {
    let mut value = HeaderValue::from_str(value).map_err(HttpError::malformed)?;
    value.set_sensitive(true);
    Ok(value)
}

/// Puts `value` on a built request as its `Authorization`, marked sensitive so the request's
/// `Debug` never shows it. A value that cannot be a header's is `Malformed`.
///
/// # Errors
/// [`HttpError::Malformed`] when `value` is not a valid header value.
pub fn authorize(request: &mut Request<Vec<u8>>, value: &str) -> Result<(), HttpError> {
    request
        .headers_mut()
        .insert(AUTHORIZATION, sensitive(value)?);
    Ok(())
}

/// The password an account presents: the credential, when it is one.
///
/// # Errors
/// `Unauthorized` when nothing is presented, `Unreadable` when the credential is not a password.
pub fn password_of(presented: &Presented) -> Result<&SecretText, ProviderError> {
    match presented {
        Presented::Credential(Credential::Password(password)) => Ok(password),
        Presented::Anonymous => Err(ProviderError::Unauthorized),
        Presented::Credential(_) => Err(ProviderError::Unreadable),
    }
}

/// The secret a generic account presents: a password, or a bearer token the person pasted.
///
/// # Errors
/// As [`password_of`]: `Unauthorized` when nothing is presented, `Unreadable` for another kind
/// of credential.
pub fn secret_presented(presented: &Presented) -> Result<&SecretText, ProviderError> {
    match presented {
        Presented::Credential(Credential::Bearer(token)) => Ok(token),
        other => password_of(other),
    }
}

#[cfg(test)]
mod tests;
