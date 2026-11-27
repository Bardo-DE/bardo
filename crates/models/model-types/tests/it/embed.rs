//! Reply checks for embeddings.

use base_ids::{Count, Dims};
use model_types::{EmbedEnd, EmbedError, EmbedVector, ModelName, TurnUsage};

fn end(widths: &[usize]) -> EmbedEnd {
    EmbedEnd {
        vectors: widths.iter().map(|w| EmbedVector(vec![0.5; *w])).collect(),
        usage: TurnUsage::default(),
        served: ModelName("m".into()),
    }
}

#[test]
fn a_reply_is_checked_for_count_then_width() {
    assert_eq!(end(&[3, 3]).check(Count(2), Dims(3)), Ok(()));
    assert_eq!(end(&[]).check(Count(0), Dims(3)), Ok(()));
    assert_eq!(
        end(&[3]).check(Count(2), Dims(3)),
        Err(EmbedError::CountMismatch {
            want: Count(2),
            got: Count(1)
        })
    );
    assert_eq!(
        end(&[3, 2, 4]).check(Count(3), Dims(3)),
        Err(EmbedError::WidthMismatch {
            want: Dims(3),
            got: Dims(2)
        })
    );
    // The count is checked before any width.
    assert_eq!(
        end(&[9]).check(Count(2), Dims(3)),
        Err(EmbedError::CountMismatch {
            want: Count(2),
            got: Count(1)
        })
    );
}
