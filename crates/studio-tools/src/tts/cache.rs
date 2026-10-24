//! Deterministic cache key generation for TTS audio synthesis.
//!
//! Hashes input parameters (text, voice, speed, noise settings, pause mappings)
//! so that any adjustment to voice, rhythm, or text produces a distinct cache key.

use hex::ToHex;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Compute a deterministic SHA-256 cache key for TTS speech synthesis.
pub fn compute_tts_cache_key(provider: &str, input: &Value) -> String {
    let mut hasher = Sha256::new();

    // Provider tag
    hasher.update(b"PROVIDER:\n");
    hasher.update(provider.as_bytes());
    hasher.update(b"\n");

    // Normalizer version
    hasher.update(b"NORM_VERSION:vi_norm_v1\n");

    // Speed algorithm version tag (V03 deterministic atempo adjustment clamped [0.7, 1.0])
    hasher.update(b"SPEED_ALGO:atempo_v1_min0.7\n");

    // Text content
    let text = input.get("text").and_then(Value::as_str).unwrap_or("");
    hasher.update(b"TEXT:\n");
    hasher.update(text.as_bytes());
    hasher.update(b"\n");

    // Voice
    let voice = input
        .get("voice")
        .and_then(Value::as_str)
        .unwrap_or("default");
    hasher.update(b"VOICE:\n");
    hasher.update(voice.as_bytes());
    hasher.update(b"\n");

    // Speed (default 0.76 for Vietnamese ~3.2 syllables/s)
    let speed = input
        .get("speed")
        .and_then(Value::as_f64)
        .map(|s| s as f32)
        .unwrap_or(0.76);
    hasher.update(b"SPEED:\n");
    hasher.update(format!("{speed:.3}").as_bytes());
    hasher.update(b"\n");

    // Noise parameters (Piper)
    if let Some(ns) = input.get("noise_scale").and_then(Value::as_f64) {
        hasher.update(format!("NOISE_SCALE:{ns:.3}\n").as_bytes());
    } else {
        hasher.update(b"NOISE_SCALE:default\n");
    }

    if let Some(nw) = input.get("noise_w").and_then(Value::as_f64) {
        hasher.update(format!("NOISE_W:{nw:.3}\n").as_bytes());
    } else {
        hasher.update(b"NOISE_W:default\n");
    }

    if let Some(ss) = input.get("sentence_silence").and_then(Value::as_f64) {
        hasher.update(format!("SENTENCE_SILENCE:{ss:.3}\n").as_bytes());
    }

    // Reference audio for cloning
    if let Some(ref_a) = input.get("reference_audio").and_then(Value::as_str) {
        hasher.update(b"REF_AUDIO:\n");
        hasher.update(ref_a.as_bytes());
        hasher.update(b"\n");
    }

    // Pauses configuration object
    if let Some(pauses) = input.get("pauses") {
        hasher.update(b"PAUSES:\n");
        hasher.update(serde_json::to_string(pauses).unwrap_or_default().as_bytes());
        hasher.update(b"\n");
    }

    hasher.finalize().encode_hex::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tts_cache_key_deterministic_and_sensitive() {
        let base_input = json!({
            "text": "Chào bạn, đây là thử nghiệm giọng đọc tự nhiên.",
            "voice": "vi_VN-vais1000-medium",
            "speed": 0.9,
            "noise_scale": 0.667,
            "noise_w": 0.85,
            "pauses": {
                "comma": 0.18,
                "period": 0.45
            }
        });

        let key1 = compute_tts_cache_key("tts.piper", &base_input);
        let key2 = compute_tts_cache_key("tts.piper", &base_input);
        assert_eq!(
            key1, key2,
            "Cache key must be deterministic for identical input"
        );

        // 1. Text changed
        let mut text_changed = base_input.clone();
        text_changed["text"] = json!("Chào bạn, đây là thử nghiệm khác.");
        assert_ne!(
            key1,
            compute_tts_cache_key("tts.piper", &text_changed),
            "Key must change when text changes"
        );

        // 2. Voice changed
        let mut voice_changed = base_input.clone();
        voice_changed["voice"] = json!("vi_VN-25hours_single-low");
        assert_ne!(
            key1,
            compute_tts_cache_key("tts.piper", &voice_changed),
            "Key must change when voice changes"
        );

        // 3. Speed changed
        let mut speed_changed = base_input.clone();
        speed_changed["speed"] = json!(0.85);
        assert_ne!(
            key1,
            compute_tts_cache_key("tts.piper", &speed_changed),
            "Key must change when speed changes"
        );

        // 4. Noise scale changed
        let mut noise_changed = base_input.clone();
        noise_changed["noise_scale"] = json!(0.75);
        assert_ne!(
            key1,
            compute_tts_cache_key("tts.piper", &noise_changed),
            "Key must change when noise_scale changes"
        );

        // 5. Noise w changed
        let mut noisew_changed = base_input.clone();
        noisew_changed["noise_w"] = json!(0.90);
        assert_ne!(
            key1,
            compute_tts_cache_key("tts.piper", &noisew_changed),
            "Key must change when noise_w changes"
        );

        // 6. Pauses changed
        let mut pauses_changed = base_input.clone();
        pauses_changed["pauses"] = json!({ "comma": 0.25, "period": 0.55 });
        assert_ne!(
            key1,
            compute_tts_cache_key("tts.piper", &pauses_changed),
            "Key must change when pauses change"
        );

        // 7. Provider changed
        assert_ne!(
            key1,
            compute_tts_cache_key("tts.vieneu", &base_input),
            "Key must change when provider changes"
        );
    }
}
