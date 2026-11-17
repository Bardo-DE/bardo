//! A streamed PCM body: whole samples out whatever the chunking, nothing lost but a partial
//! trailing sample.
#![no_main]

use libfuzzer_sys::fuzz_target;
use model_fuzz::{pieces, split};
use model_openai_compat::{KOKORO_FORMAT, PcmDecoder};

fuzz_target!(|data: &[u8]| {
    let (cuts, bytes) = split(data);
    let mut decoder = PcmDecoder::new(KOKORO_FORMAT);
    let got: usize = pieces(bytes, &cuts)
        .iter()
        .filter_map(|c| decoder.feed(c))
        .map(|chunk| chunk.pcm.len())
        .sum();
    assert_eq!(got, bytes.len() - bytes.len() % 2);
});
