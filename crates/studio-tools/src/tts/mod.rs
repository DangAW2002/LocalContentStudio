//! Text-to-Speech (TTS) enhancements for natural speech rhythm.
//!
//! Includes Vietnamese text normalization, clause/punctuation segmentation with
//! pauses, deterministic pure-Rust PCM WAV concatenation, and cache key computation.

pub mod cache;
pub mod norm;
pub mod segment;
pub mod speed;
pub mod wav;

pub use cache::compute_tts_cache_key;
pub use norm::normalize_vietnamese_text;
pub use segment::{split_into_tts_segments, PauseConfig, TtsSegment};
pub use speed::{
    apply_atempo_filter, calculate_atempo, compute_effective_speed, compute_syllables_per_second,
    count_vietnamese_syllables,
};
pub use wav::{concat_wav_files_with_silence, read_pcm_wav, write_pcm_wav, WavSpec};
