//! Batch planning for embeddings.

use model_core::plan_batches;
use model_types::BatchMax;
use proptest::prelude::*;

proptest! {
    #[test]
    fn batches_cover_the_inputs_exactly_once_in_order(n in 0usize..5000, max in 0u32..70) {
        let ranges = plan_batches(n, BatchMax(max));
        let step = max.max(1) as usize;
        let mut next = 0;
        for range in &ranges {
            prop_assert_eq!(range.start, next);
            prop_assert!(range.end > range.start && range.end - range.start <= step);
            next = range.end;
        }
        prop_assert_eq!(next, n);
        prop_assert_eq!(plan_batches(n, BatchMax(max)), ranges);
    }
}
