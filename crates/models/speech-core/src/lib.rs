//! Speech as traits, below every engine and account, local or cloud.
//!
//! Audio is mono only (`PipeWire` converts at the edge). [`SpeechToText`] pulls audio from an
//! [`AudioSource`] and pushes [`TranscriptEvent`]s into a [`TranscriptSink`]; [`TextToSpeech`]
//! pushes [`AudioChunk`]s into an [`AudioSink`]. Cancellation is dropping the future, as in
//! `model-core`. [`VoiceActivity`] is synchronous: CPU only, one 32 ms frame at a time.
//! `host_wire` is the framed JSON between inferd and the `bardo-speech-host` engine process.
//!
//! What a person said, what a model heard and every sample are personal: `PcmBytes`, `HeardText`
//! and `SpokenText` write `Debug` by hand and print a length, never the content.
//!
//! ```
//! use speech_core::{HostIn, decode_frame, encode_frame, frame_length};
//!
//! // One frame of the host wire: a 4-byte length, then the JSON body.
//! let frame = encode_frame(&HostIn::End).unwrap();
//! let header: [u8; 4] = frame[..4].try_into().unwrap();
//! let len = frame_length(header).unwrap();
//! let back: HostIn = decode_frame(&frame[4..4 + len]).unwrap();
//! assert_eq!(back, HostIn::End);
//! ```
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

mod audio;
mod caps;
mod host_wire;
mod stt;
mod text;
mod tts;
mod vad;

pub use audio::{AudioChunk, AudioFormat, AudioMs, PcmBytes, PcmFormat, SampleIndex, SampleRate};
pub use caps::{LangSet, SpeechCaps, SpeechDir, SpeechIo, SpeechModelInfo};
pub use host_wire::{
    FrameError, HostIn, HostOut, HostVocab, MAX_FRAME_BYTES, decode_frame, encode_frame,
    frame_length,
};
pub use stt::{
    AudioSource, SpeechToText, SttEnd, SttMode, SttRequest, TranscriptEvent, TranscriptSink,
};
pub use text::{
    HeardText, Lang, LangChoice, LangError, SpokenText, SpokenTextError, VoiceId, VoiceIdError,
};
pub use tts::{AudioSink, TextToSpeech, TtsEnd, TtsRequest};
pub use vad::{FRAME_SAMPLES, Frame512, FrameLenError, SpeechProb, VoiceActivity, Voiced};
