//! What a status the replica did not expect means to the contract: a read that failed
//! ([`ReplicaError`]) or a write that was refused ([`PutError`]). Drive throttles with `429`,
//! `503` and, as a `403`, the reasons `rateLimitExceeded` and `userRateLimitExceeded`; a full
//! account is a `403 storageQuotaExceeded`. A day's limit of requests or uploads used up
//! (`dailyLimitExceeded`, `quotaExceeded`) is waited out for [`DAY_LIMIT`].

use crate::json::ErrorBody;
use base_time::UnixSeconds;
use http_core::http::Response;
use sync_core::status::{self, retry_after};
use sync_core::{ExportError, PutError, ReplicaError, RetryAfter};

/// How long to wait when a day's limit is used up: an hour, then again.
pub const DAY_LIMIT: RetryAfter = RetryAfter(3600);

/// Whether a refusal is a day's limit used up.
fn day_limit(reason: Option<&str>) -> bool {
    matches!(reason, Some("dailyLimitExceeded" | "quotaExceeded"))
}

/// The first reason an error body gives.
fn reason(response: &Response<Vec<u8>>) -> Option<String> {
    let body: ErrorBody = serde_json::from_slice(response.body()).ok()?;
    body.error?.errors.into_iter().find_map(|r| r.reason)
}

/// Whether a `403` is a throttle rather than a refusal.
fn throttled(reason: Option<&str>) -> bool {
    matches!(
        reason,
        Some("rateLimitExceeded" | "userRateLimitExceeded" | "sharingRateLimitExceeded")
    )
}

/// A read answered with `response`, which was not what it needed.
pub fn read_error(response: &Response<Vec<u8>>, now: UnixSeconds) -> ReplicaError {
    match response.status().as_u16() {
        403 | 429 if day_limit(reason(response).as_deref()) => ReplicaError::Transient(DAY_LIMIT),
        403 if throttled(reason(response).as_deref()) => {
            ReplicaError::Transient(retry_after(response, now))
        }
        _ => status::read_error(response, now),
    }
}

/// An export answered with `response`, which was not the file: Drive refuses a document past
/// its export limit with `403 exportSizeLimitExceeded`, and one with no file format (a Form, a
/// Site) with `403 cannotExportFile` or a `400`.
pub fn export_error(response: &Response<Vec<u8>>, now: UnixSeconds) -> ExportError {
    match (response.status().as_u16(), reason(response).as_deref()) {
        (403, Some("exportSizeLimitExceeded")) => ExportError::TooLarge,
        (403, Some("cannotExportFile" | "fileNotExportable")) | (400, _) => {
            ExportError::Unsupported
        }
        _ => ExportError::Read(read_error(response, now)),
    }
}

/// A write answered with `response`, which was not a success or a precondition failure.
pub fn write_error(response: &Response<Vec<u8>>, now: UnixSeconds) -> PutError {
    let why = reason(response);
    match response.status().as_u16() {
        403 if why.as_deref() == Some("storageQuotaExceeded") => PutError::Quota,
        403 | 429 if day_limit(why.as_deref()) => PutError::Transient(DAY_LIMIT),
        403 if throttled(why.as_deref()) => PutError::Transient(retry_after(response, now)),
        401 => PutError::Unauthorized,
        409 => PutError::Declined,
        400 | 403 | 405 | 413 => PutError::Forbidden,
        _ => PutError::Transient(retry_after(response, now)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sync_core::status::RETRY;

    fn answer(status: u16, retry: Option<&str>, reason: Option<&str>) -> Response<Vec<u8>> {
        let body = reason
            .map_or_default(|r| format!(r#"{{"error":{{"errors":[{{"reason":"{r}"}}]}}}}"#).into_bytes());
        retry
            .into_iter()
            .fold(Response::builder().status(status), |b, v| {
                b.header("Retry-After", v)
            })
            .body(body)
            .expect("response")
    }

    #[test]
    fn a_refused_export_says_too_large_no_format_or_why_it_failed() {
        let cases = [
            (403, Some("exportSizeLimitExceeded"), ExportError::TooLarge),
            (403, Some("cannotExportFile"), ExportError::Unsupported),
            (400, Some("badRequest"), ExportError::Unsupported),
            (404, None, ExportError::Read(ReplicaError::Gone)),
            (
                403,
                Some("insufficientPermissions"),
                ExportError::Read(ReplicaError::Unauthorized),
            ),
            (503, None, ExportError::Read(ReplicaError::Transient(RETRY))),
        ];
        for (status, why, want) in cases {
            assert_eq!(
                export_error(&answer(status, None, why), UnixSeconds(0)),
                want,
                "{status} {why:?}"
            );
        }
    }

    #[test]
    fn a_failed_read_is_classed_by_its_status_and_reason() {
        const CASES: &[(u16, Option<&str>, Option<&str>, ReplicaError)] = &[
            (401, None, None, ReplicaError::Unauthorized),
            (
                403,
                None,
                Some("insufficientPermissions"),
                ReplicaError::Unauthorized,
            ),
            (
                403,
                None,
                Some("rateLimitExceeded"),
                ReplicaError::Transient(RETRY),
            ),
            (404, None, None, ReplicaError::Gone),
            (
                429,
                Some("17"),
                None,
                ReplicaError::Transient(RetryAfter(17)),
            ),
            (
                503,
                Some("120"),
                None,
                ReplicaError::Transient(RetryAfter(120)),
            ),
            (500, None, None, ReplicaError::Transient(RETRY)),
        ];
        for (status, retry, reason, want) in CASES {
            let got = read_error(&answer(*status, *retry, *reason), UnixSeconds(0));
            assert!(got == *want, "{status} {reason:?}: {got:?}");
        }
    }

    #[test]
    fn a_refused_write_is_classed_by_its_status_and_reason() {
        const CASES: &[(u16, Option<&str>, Option<&str>, PutError)] = &[
            (403, None, Some("storageQuotaExceeded"), PutError::Quota),
            (
                403,
                None,
                Some("insufficientFilePermissions"),
                PutError::Forbidden,
            ),
            (
                403,
                Some("9"),
                Some("userRateLimitExceeded"),
                PutError::Transient(RetryAfter(9)),
            ),
            (400, None, None, PutError::Forbidden),
            (401, None, None, PutError::Unauthorized),
            (409, None, None, PutError::Declined),
            (413, None, None, PutError::Forbidden),
            (
                403,
                None,
                Some("dailyLimitExceeded"),
                PutError::Transient(DAY_LIMIT),
            ),
            (
                403,
                None,
                Some("quotaExceeded"),
                PutError::Transient(DAY_LIMIT),
            ),
            (429, Some("7"), None, PutError::Transient(RetryAfter(7))),
            (502, None, None, PutError::Transient(RETRY)),
        ];
        for (status, retry, reason, want) in CASES {
            let got = write_error(&answer(*status, *retry, *reason), UnixSeconds(0));
            assert!(got == *want, "{status} {reason:?}: {got:?}");
        }
    }

    #[test]
    fn a_days_limit_used_up_is_waited_out_for_an_hour() {
        assert_eq!(
            read_error(
                &answer(403, None, Some("dailyLimitExceeded")),
                UnixSeconds(0)
            ),
            ReplicaError::Transient(DAY_LIMIT)
        );
    }
}
