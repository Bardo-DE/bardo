//! What a status the replica did not expect means to the contract: a write that was refused
//! ([`PutError`]). The statuses every HTTP replica reads alike are sync-core's `status`.

use base_time::UnixSeconds;
use http_core::http::Response;
use replica_http::retry_after;
use sync_core::PutError;

/// A write answered with `response`, which was not a success or a precondition failure.
pub fn write_error(response: &Response<Vec<u8>>, now: UnixSeconds) -> PutError {
    match response.status().as_u16() {
        507 => PutError::Quota,
        401 | 403 | 405 | 409 | 413 | 423 => PutError::Forbidden,
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
            (403, None, PutError::Forbidden),
            (423, None, PutError::Forbidden),
            (409, None, PutError::Forbidden),
            (503, Some("7"), PutError::Transient(RetryAfter(7))),
            (502, None, PutError::Transient(RETRY)),
        ];
        for (status, retry, want) in CASES {
            let got = write_error(&answer(*status, *retry), UnixSeconds(0));
            assert!(got == *want, "{status}: {got:?}");
        }
    }
}
