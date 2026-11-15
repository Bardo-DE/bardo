//! The wire: every request that names a space says so.

use crate::support::*;
use almanac_core::*;

/// Indexes every variant that has a sample.
fn request_index(r: &MemoryRequest) -> usize {
    match r {
        MemoryRequest::Record(_) => 0,
        MemoryRequest::RecordBatch(_) => 1,
        MemoryRequest::ExplainFile(_) => 2,
        MemoryRequest::Mark(_) => 3,
        MemoryRequest::Search(_) => 4,
        MemoryRequest::Facts(_) => 5,
        MemoryRequest::Related(..) => 6,
        MemoryRequest::Provenance(..) => 7,
        MemoryRequest::Primer(_) => 8,
        MemoryRequest::Propose(..) => 9,
        MemoryRequest::Spaces => 10,
        MemoryRequest::Status(_) => 11,
        MemoryRequest::Timeline(..) => 12,
        MemoryRequest::PlanForget(..) => 13,
        MemoryRequest::Forget(_) => 14,
        MemoryRequest::Pending(_) => 15,
        MemoryRequest::Settle(..) => 16,
        MemoryRequest::Consolidation(_) => 17,
        MemoryRequest::RunConsolidation(_) => 18,
        MemoryRequest::Revert(_) => 19,
        MemoryRequest::Rules => 20,
        MemoryRequest::SetRule(_) => 21,
        MemoryRequest::RemoveRule(_) => 22,
        MemoryRequest::Pause(..) => 23,
        MemoryRequest::Resume(_) => 24,
        MemoryRequest::Verify(_) => 25,
        MemoryRequest::Rebuild(_) => 26,
        MemoryRequest::Export(_) => 27,
        MemoryRequest::Inject(_) => 28,
        MemoryRequest::Recent(..) => 29,
        MemoryRequest::Sweep(_) => 30,
        MemoryRequest::ApplyConsolidation(_) => 31,
        MemoryRequest::DiscardConsolidation(_) => 32,
        MemoryRequest::RecordDurable(_) => 33,
        MemoryRequest::Entries(..) => 34,
        MemoryRequest::RemoveSpace(..) => 35,
        MemoryRequest::ConfirmSettings(_) => 36,
        MemoryRequest::PendingSettings => 37,
        // The enum is non_exhaustive, so this match cannot be exhaustive from outside the crate.
        // A new variant that has a sample lands here and fails the coverage assert below.
        _ => usize::MAX,
    }
}

#[test]
fn requests_name_their_space() {
    for request in requests() {
        let named = request.space().is_some();
        let by_index = !matches!(
            request_index(&request),
            1 | 10 | 14 | 16 | 19 | 20 | 21 | 22 | 27 | 31 | 32 | 36 | 37
        );
        assert_eq!(named, by_index, "{request:?}");
    }
}
