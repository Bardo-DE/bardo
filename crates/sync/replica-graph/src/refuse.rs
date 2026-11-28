//! What a status the replica did not expect means to the contract: a read that failed
//! ([`ReplicaError`]) or a write that was refused ([`PutError`]). Graph throttles with `429`
//! and `503` and a `Retry-After` in seconds, and answers `423` for an item locked on the drive
//! (open in an Office app there, or checked out).

use crate::json::ErrorBody;
use base_time::UnixSeconds;
use http_core::http::Response;
use replica_http::retry_after;
use sync_core::PutError;

/// The code an error's answer gives (`nameAlreadyExists`, `invalidRequest`).
pub fn error_code(response: &Response<Vec<u8>>) -> Option<String> {
    let body: ErrorBody = serde_json::from_slice(response.body()).ok()?;
    body.error?.code
}

/// A write answered with `response`, which was not a success or a precondition failure.
pub fn write_error(response: &Response<Vec<u8>>, now: UnixSeconds) -> PutError {
    match response.status().as_u16() {
        507 => PutError::Quota,
        401 => PutError::Unauthorized,
        423 => PutError::InUse(retry_after(response, now)),
        409 => PutError::Declined,
        400 | 403 | 405 | 413 => PutError::Forbidden,
        _ => PutError::Transient(retry_after(response, now)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use replica_http::RETRY;
    use sync_core::RetryAfter;

    fn answer(status: u16, retry: Option<&str>) -> Response<Vec<u8>> {
        retry
            .into_iter()
            .fold(Response::builder().status(status), |b, v| {
                b.header("Retry-After", v)
            })
            .body(vec![])
            .expect("response")
    }

    #[test]
    fn a_refused_write_is_classed_by_its_status() {
        const CASES: &[(u16, Option<&str>, PutError)] = &[
            (507, None, PutError::Quota),
            (401, None, PutError::Unauthorized),
            (403, None, PutError::Forbidden),
            (409, None, PutError::Declined),
            (413, None, PutError::Forbidden),
            (423, None, PutError::InUse(RETRY)),
            (423, Some("12"), PutError::InUse(RetryAfter(12))),
            (429, Some("7"), PutError::Transient(RetryAfter(7))),
            (502, None, PutError::Transient(RETRY)),
        ];
        for (status, retry, want) in CASES {
            let got = write_error(&answer(*status, *retry), UnixSeconds(0));
            assert!(got == *want, "{status}: {got:?}");
        }
    }
}
