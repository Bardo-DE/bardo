//! What a host sends is hostile input: any bytes, in any chunking, end in frames or a typed
//! error, and a header over the cap is refused before its body is waited for.

use model_types::ProviderError;
use proptest::prelude::*;
use speech_core::{HostOut, MAX_FRAME_BYTES};
use speech_host_client::FrameBuffer;

/// Every frame the bytes hold, or the error that stopped the reading.
fn read(bytes: &[u8], cuts: &[usize]) -> (Vec<HostOut>, Option<ProviderError>) {
    let mut points: Vec<usize> = cuts.iter().map(|c| c % (bytes.len() + 1)).collect();
    points.extend([0, bytes.len()]);
    points.sort_unstable();
    let mut buffer = FrameBuffer::default();
    let mut frames = Vec::new();
    for w in points.windows(2) {
        buffer.push(&bytes[w[0]..w[1]]);
        loop {
            match buffer.take() {
                Ok(Some(frame)) => frames.push(frame),
                Ok(None) => break,
                Err(error) => return (frames, Some(error)),
            }
        }
    }
    (frames, None)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn arbitrary_bytes_end_in_frames_or_a_typed_error(
        bytes in proptest::collection::vec(any::<u8>(), 0..400),
        cuts in proptest::collection::vec(any::<usize>(), 0..8),
    ) {
        let (split, split_error) = read(&bytes, &cuts);
        let (whole, whole_error) = read(&bytes, &[]);
        prop_assert_eq!(split, whole);
        prop_assert_eq!(split_error, whole_error);
    }
}

#[test]
fn a_header_over_the_cap_is_refused_before_the_body_arrives() {
    let mut buffer = FrameBuffer::default();
    buffer.push(&u32::try_from(MAX_FRAME_BYTES + 1).unwrap().to_be_bytes());
    assert!(matches!(buffer.take(), Err(ProviderError::Unreadable(_))));
    let mut buffer = FrameBuffer::default();
    buffer.push(&u32::MAX.to_be_bytes());
    assert!(matches!(buffer.take(), Err(ProviderError::Unreadable(_))));
}
