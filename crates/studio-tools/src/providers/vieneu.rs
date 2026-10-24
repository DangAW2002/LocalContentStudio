//! VieNeu-TTS provider (v3 Turbo, Apache-2.0, ONNX on CPU).
//!
//! Synthesizes speech offline using VieNeu-TTS via a small bundled Python helper script.
//! Input text is always written to a temporary UTF-8 file and never passed via shell arguments.

use crate::error::{Result, ToolError};
use crate::probe::{find_in_path, ProbeResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use studio_workspace::Workspace;

/// Bundled Python runner script content.
pub const VIENEU_HELPER_SCRIPT: &str =
    include_str!("../../builtin/tools/tts.vieneu/vieneu_helper.py");

/// Available default preset voices for VieNeu-TTS v3 Turbo.
pub const VIENEU_DEFAULT_VOICES: &[&str] = &[
    "Thanh Bình",
    "Mai Anh",
    "Minh Đức",
    "Trúc Ly",
    "Minh Triết",
    "Mỹ Duyên",
    "Quỳnh Anh",
    "Đức Trí",
    "Kim Thanh",
    "Adam",
    "Quốc Tuấn",
];

/// Options for configuring VieNeu-TTS synthesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VieneuVoiceConfig {
    /// Voice preset name (e.g., "Bình", "Mai", "Nam", "Nữ").
    pub voice: String,
    /// Speed multiplier (default 0.76 for Vietnamese ~3.2 syllables/s).
    pub speed: f32,
    /// Optional workspace-relative reference audio file for voice cloning.
    pub reference_audio: Option<String>,
}

impl Default for VieneuVoiceConfig {
    fn default() -> Self {
        Self {
            voice: "Bình".to_string(),
            speed: 0.76,
            reference_audio: None,
        }
    }
}

/// Probe system for Python and the `vieneu` package.
///
/// Executes `python -c "import vieneu; print(getattr(vieneu, '__version__', 'v3-turbo'))"`
/// and returns a structured ProbeResult without fabricating success.
pub fn probe_vieneu() -> ProbeResult {
    let py_bin = find_in_path("python").or_else(|| find_in_path("python3"));
    let Some(py_exe) = py_bin else {
        return ProbeResult {
            available: false,
            missing: vec!["bin:python".into()],
            note_vi: "Chưa cài đặt Python. Vui lòng cài đặt Python 3.10+ từ https://python.org hoặc qua winget: 'winget install Python.Python.3.11'".into(),
        };
    };

    let mut cmd = Command::new(&py_exe);
    cmd.args([
        "-c",
        "import vieneu; print(getattr(vieneu, '__version__', 'v3-turbo'))",
    ]);

    match cmd.output() {
        Ok(out) if out.status.success() => {
            let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let display_ver = if ver.is_empty() { "v3-turbo" } else { &ver };
            ProbeResult {
                available: true,
                missing: Vec::new(),
                note_vi: format!("Sẵn sàng (VieNeu-TTS {display_ver})"),
            }
        }
        _ => ProbeResult {
            available: false,
            missing: vec!["python:vieneu".into()],
            note_vi: "Chưa cài đặt thư viện 'vieneu'. Hãy chạy: pip install vieneu (hoặc pip install vieneu --extra-index-url https://pnnbao97.github.io/llama-cpp-python-v0.3.16/cpu/)".into(),
        },
    }
}

/// Construct CLI arguments for invoking the helper script.
pub fn build_vieneu_args(
    script_path: &Path,
    text_file_path: &Path,
    output_file_path: &Path,
    voice: Option<&str>,
    speed: Option<f32>,
    ref_audio: Option<&Path>,
    meta_file: Option<&Path>,
) -> Vec<String> {
    let mut args = vec![
        "-u".to_string(),
        script_path.to_string_lossy().to_string(),
        "--text-file".to_string(),
        text_file_path.to_string_lossy().to_string(),
        "--output-file".to_string(),
        output_file_path.to_string_lossy().to_string(),
    ];

    if let Some(v) = voice {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            args.push("--voice".to_string());
            args.push(trimmed.to_string());
        }
    }

    if let Some(s) = speed {
        if (s - 1.0).abs() > f32::EPSILON && s > 0.1 && s < 5.0 {
            args.push("--speed".to_string());
            args.push(format!("{s:.2}"));
        }
    }

    if let Some(r) = ref_audio {
        args.push("--ref-audio".to_string());
        args.push(r.to_string_lossy().to_string());
    }

    if let Some(m) = meta_file {
        args.push("--metadata-file".to_string());
        args.push(m.to_string_lossy().to_string());
    }

    args
}

/// Ensure the bundled python helper script is written to a designated cache file.
pub fn ensure_helper_script(cache_dir: &Path) -> Result<PathBuf> {
    let script_dir = cache_dir.join("vieneu");
    fs::create_dir_all(&script_dir).map_err(|e| ToolError::Io {
        path: script_dir.clone(),
        source: e,
    })?;

    let script_path = script_dir.join("vieneu_helper.py");
    fs::write(&script_path, VIENEU_HELPER_SCRIPT).map_err(|e| ToolError::Io {
        path: script_path.clone(),
        source: e,
    })?;

    Ok(script_path)
}

/// Run VieNeu-TTS synthesis in-process via Python helper with natural punctuation pausing,
/// Vietnamese text normalization, and deterministic cache.
pub async fn run_vieneu(ws: &Workspace, out_dir: &Path, input: &Value) -> Result<Value> {
    let probe_res = probe_vieneu();
    if !probe_res.available {
        return Err(ToolError::ExecutionFailed {
            tool_id: "tts.vieneu".into(),
            reason: probe_res.note_vi,
        });
    }

    let text = input
        .get("text")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| {
            ToolError::ParseError("Thiếu tham số 'text' để tổng hợp giọng nói".into())
        })?;

    let voice_req = input.get("voice").and_then(Value::as_str);
    // Default speed for Vietnamese is 0.76 (calibrated for target ~3.2 syllables/s) if not specified
    let speed_req = input
        .get("speed")
        .and_then(Value::as_f64)
        .map(|s| s as f32)
        .unwrap_or(0.76);

    let actual_voice = voice_req.unwrap_or("Bình").to_string();

    let ref_audio_scoped: Option<PathBuf> =
        if let Some(ref_str) = input.get("reference_audio").and_then(Value::as_str) {
            let trimmed = ref_str.trim();
            if !trimmed.is_empty() {
                let scoped = ws.scoped_path(trimmed).map_err(|e| ToolError::Io {
                    path: PathBuf::from(trimmed),
                    source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, e),
                })?;
                if !scoped.is_file() {
                    return Err(ToolError::ExecutionFailed {
                        tool_id: "tts.vieneu".into(),
                        reason: format!("Tệp reference_audio không tồn tại: {trimmed}"),
                    });
                }
                Some(scoped)
            } else {
                None
            }
        } else {
            None
        };

    let target_wav_path: PathBuf =
        if let Some(out_req) = input.get("output_path").and_then(Value::as_str) {
            let scoped = ws.scoped_path(out_req).map_err(|e| ToolError::Io {
                path: PathBuf::from(out_req),
                source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, e),
            })?;
            if let Some(parent) = scoped.parent() {
                let _ = fs::create_dir_all(parent);
            }
            scoped
        } else {
            let file_id = uuid::Uuid::new_v4().to_string();
            let path = out_dir.join(format!("vieneu-{file_id}.wav"));
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            path
        };

    // Check TTS audio cache by input hash
    let cache_key = crate::tts::compute_tts_cache_key("tts.vieneu", input);
    let cache_dir = ws.cache_dir().join("tts");
    let cached_file = cache_dir.join(format!("{cache_key}.wav"));

    if cached_file.is_file() {
        let _ = fs::copy(&cached_file, &target_wav_path);
        let duration_s = if let Ok((spec, data)) = crate::tts::read_pcm_wav(&target_wav_path) {
            spec.duration_seconds(data.len())
        } else {
            0.0
        };

        let rel_path = target_wav_path
            .strip_prefix(ws.root())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| target_wav_path.to_string_lossy().replace('\\', "/"));

        let meta_file = cache_dir.join(format!("{cache_key}.meta.json"));
        let (cached_eff_speed, cached_sps) = if let Ok(content) = fs::read_to_string(&meta_file) {
            if let Ok(meta_json) = serde_json::from_str::<Value>(&content) {
                let eff = meta_json
                    .get("effective_speed")
                    .and_then(Value::as_f64)
                    .map(|v| v as f32)
                    .unwrap_or(speed_req);
                let sps = meta_json
                    .get("syllables_per_second")
                    .and_then(Value::as_f64)
                    .map(|v| v as f32)
                    .unwrap_or(0.0);
                (eff, sps)
            } else {
                let norm = crate::tts::normalize_vietnamese_text(text);
                let syl = crate::tts::count_vietnamese_syllables(&norm);
                (
                    speed_req,
                    crate::tts::compute_syllables_per_second(syl, duration_s),
                )
            }
        } else {
            let norm = crate::tts::normalize_vietnamese_text(text);
            let syl = crate::tts::count_vietnamese_syllables(&norm);
            (
                speed_req,
                crate::tts::compute_syllables_per_second(syl, duration_s),
            )
        };

        return Ok(json!({
            "audio_path": rel_path,
            "duration_seconds": (duration_s * 1000.0).round() / 1000.0,
            "provider": "tts.vieneu",
            "voice": actual_voice,
            "speed": speed_req,
            "effective_speed": cached_eff_speed,
            "syllables_per_second": cached_sps,
            "available_voices": VIENEU_DEFAULT_VOICES,
            "cached": true,
            "note": "Tổng hợp giọng nói thành công qua VieNeu-TTS v3 Turbo (bản lưu cache)."
        }));
    }

    let py_exe = find_in_path("python")
        .or_else(|| find_in_path("python3"))
        .ok_or_else(|| ToolError::MissingDependency {
            tool_id: "tts.vieneu".into(),
            missing: vec!["bin:python".into()],
        })?;

    let script_path = ensure_helper_script(&ws.cache_dir())?;

    // Step 1: Normalize Vietnamese text
    let normalized = crate::tts::normalize_vietnamese_text(text);

    // Step 2: Split into speech segments with punctuation pauses
    let pauses = crate::tts::PauseConfig::from_value(input.get("pauses"));
    let segments = crate::tts::split_into_tts_segments(&normalized, &pauses);

    if segments.is_empty() {
        return Err(ToolError::ExecutionFailed {
            tool_id: "tts.vieneu".into(),
            reason: "Nội dung văn bản sau chuẩn hoá rỗng.".into(),
        });
    }

    let chunks_dir = ws
        .cache_dir()
        .join("vieneu_chunks")
        .join(uuid::Uuid::new_v4().to_string());
    fs::create_dir_all(&chunks_dir).map_err(|e| ToolError::Io {
        path: chunks_dir.clone(),
        source: e,
    })?;

    let mut chunk_items: Vec<(PathBuf, f32)> = Vec::new();
    let mut total_base_duration = 0.0;
    let mut total_spoken_duration = 0.0;

    for (idx, seg) in segments.iter().enumerate() {
        let chunk_wav = chunks_dir.join(format!("chunk_{idx}.wav"));

        // Create a temporary file for the segment text (UTF-8, avoiding shell quoting)
        let mut temp_txt = tempfile::Builder::new()
            .prefix("vieneu-input-")
            .suffix(".txt")
            .tempfile()
            .map_err(|e| ToolError::Io {
                path: PathBuf::from("tempfile"),
                source: e,
            })?;

        temp_txt
            .write_all(seg.text.as_bytes())
            .map_err(|e| ToolError::Io {
                path: PathBuf::from("tempfile"),
                source: e,
            })?;
        temp_txt.flush().map_err(|e| ToolError::Io {
            path: PathBuf::from("tempfile"),
            source: e,
        })?;

        let meta_file_path = ws
            .cache_dir()
            .join(format!("vieneu-meta-{}.json", uuid::Uuid::new_v4()));

        let cmd_args = build_vieneu_args(
            &script_path,
            temp_txt.path(),
            &chunk_wav,
            voice_req,
            Some(speed_req),
            ref_audio_scoped.as_deref(),
            Some(&meta_file_path),
        );

        let output = Command::new(&py_exe)
            .args(&cmd_args)
            .current_dir(ws.root())
            .output()
            .map_err(|e| ToolError::ExecutionFailed {
                tool_id: "tts.vieneu".into(),
                reason: format!("Không thể khởi chạy tiến trình Python: {e}"),
            })?;

        let _ = fs::remove_file(&meta_file_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let _ = fs::remove_dir_all(&chunks_dir);
            let reason = if stderr.contains("No module named 'vieneu'") {
                "Chưa cài đặt thư viện 'vieneu'. Vui lòng chạy lệnh: pip install vieneu".to_string()
            } else if !stderr.trim().is_empty() {
                stderr.trim().to_string()
            } else {
                format!(
                    "Tiến trình VieNeu-TTS kết thúc với mã lỗi {}",
                    output.status
                )
            };
            return Err(ToolError::ExecutionFailed {
                tool_id: "tts.vieneu".into(),
                reason,
            });
        }

        if !chunk_wav.is_file() {
            let _ = fs::remove_dir_all(&chunks_dir);
            return Err(ToolError::ExecutionFailed {
                tool_id: "tts.vieneu".into(),
                reason: format!(
                    "VieNeu-TTS kết thúc nhưng không tạo được tệp âm thanh phân đoạn #{idx}."
                ),
            });
        }

        let (spec_raw, data_raw) = crate::tts::read_pcm_wav(&chunk_wav)?;
        let dur_raw = spec_raw.duration_seconds(data_raw.len());
        total_base_duration += dur_raw;

        if let Some(tempo) = crate::tts::calculate_atempo(dur_raw, speed_req, dur_raw) {
            let chunk_stretched = chunks_dir.join(format!("chunk_{idx}_stretched.wav"));
            crate::tts::apply_atempo_filter(&chunk_wav, &chunk_stretched, tempo)?;
            let (spec_str, data_str) = crate::tts::read_pcm_wav(&chunk_stretched)?;
            let dur_str = spec_str.duration_seconds(data_str.len());
            total_spoken_duration += dur_str;
            chunk_items.push((chunk_stretched, seg.pause_after_s));
        } else {
            total_spoken_duration += dur_raw;
            chunk_items.push((chunk_wav, seg.pause_after_s));
        }
    }

    // Step 3: Concatenate all segments with deterministic silence gaps
    let item_refs: Vec<(&Path, f32)> = chunk_items.iter().map(|(p, s)| (p.as_path(), *s)).collect();
    let total_duration = crate::tts::concat_wav_files_with_silence(&item_refs, &target_wav_path)?;

    // Clean up temporary chunks
    let _ = fs::remove_dir_all(&chunks_dir);

    // Compute rhythm & speed metrics
    let total_syllables = crate::tts::count_vietnamese_syllables(&normalized);
    let effective_speed =
        crate::tts::compute_effective_speed(total_base_duration, total_spoken_duration);
    let syllables_per_sec =
        crate::tts::compute_syllables_per_second(total_syllables, total_duration);

    // Step 4: Write to cache along with sidecar metadata
    let _ = fs::create_dir_all(&cache_dir);
    let _ = fs::copy(&target_wav_path, &cached_file);
    let meta_file = cache_dir.join(format!("{cache_key}.meta.json"));
    let meta_json = json!({
        "effective_speed": effective_speed,
        "syllables_per_second": syllables_per_sec,
        "total_syllables": total_syllables,
        "duration_seconds": total_duration,
    });
    let _ = fs::write(&meta_file, meta_json.to_string());

    let rel_path = target_wav_path
        .strip_prefix(ws.root())
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| target_wav_path.to_string_lossy().replace('\\', "/"));

    Ok(json!({
        "audio_path": rel_path,
        "duration_seconds": total_duration,
        "provider": "tts.vieneu",
        "voice": actual_voice,
        "speed": speed_req,
        "effective_speed": effective_speed,
        "syllables_per_second": syllables_per_sec,
        "available_voices": VIENEU_DEFAULT_VOICES,
        "cached": false,
        "note": "Tổng hợp giọng nói thành công qua VieNeu-TTS v3 Turbo."
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_build_vieneu_args_comprehensive() {
        let script = Path::new("C:/app/vieneu_helper.py");
        let text_file = Path::new("C:/temp/input.txt");
        let out_file = Path::new("C:/ws/out.wav");
        let ref_audio = Path::new("C:/ws/audio/sample.wav");
        let meta_file = Path::new("C:/temp/meta.json");

        let args = build_vieneu_args(
            script,
            text_file,
            out_file,
            Some("Mai"),
            Some(1.25),
            Some(ref_audio),
            Some(meta_file),
        );

        assert_eq!(args[0], "-u");
        assert_eq!(args[1], script.to_string_lossy());
        assert_eq!(args[2], "--text-file");
        assert_eq!(args[3], text_file.to_string_lossy());
        assert_eq!(args[4], "--output-file");
        assert_eq!(args[5], out_file.to_string_lossy());

        assert!(args.contains(&"--voice".to_string()));
        let v_idx = args.iter().position(|a| a == "--voice").unwrap();
        assert_eq!(args[v_idx + 1], "Mai");

        assert!(args.contains(&"--speed".to_string()));
        let s_idx = args.iter().position(|a| a == "--speed").unwrap();
        assert_eq!(args[s_idx + 1], "1.25");

        assert!(args.contains(&"--ref-audio".to_string()));
        let r_idx = args.iter().position(|a| a == "--ref-audio").unwrap();
        assert_eq!(args[r_idx + 1], ref_audio.to_string_lossy());

        assert!(args.contains(&"--metadata-file".to_string()));
        let m_idx = args.iter().position(|a| a == "--metadata-file").unwrap();
        assert_eq!(args[m_idx + 1], meta_file.to_string_lossy());
    }

    #[test]
    fn test_utf8_temp_file_handling() {
        let vietnamese_text =
            "Xin chào! Đây là thử nghiệm tổng hợp tiếng Việt với đầy đủ dấu: ắ, ằ, ẳ, ẵ, ặ, ơ, ư.";
        let mut temp_txt = tempfile::Builder::new()
            .prefix("test-vieneu-")
            .suffix(".txt")
            .tempfile()
            .unwrap();

        temp_txt.write_all(vietnamese_text.as_bytes()).unwrap();
        temp_txt.flush().unwrap();

        let read_back = fs::read_to_string(temp_txt.path()).unwrap();
        assert_eq!(read_back, vietnamese_text);
    }

    #[test]
    fn test_scoped_reference_audio_refuses_traversal() {
        let tmp = tempdir().unwrap();
        let ws = Workspace::create(tmp.path().join("my-ws"), "my-ws").unwrap();

        let input = json!({
            "text": "Kiểm tra",
            "reference_audio": "../outside.wav"
        });

        let ref_res = ws.scoped_path(input["reference_audio"].as_str().unwrap());
        assert!(ref_res.is_err(), "Must reject path traversal");
    }

    #[test]
    #[ignore]
    fn test_real_vieneu_synthesis() {
        let probe = probe_vieneu();
        if !probe.available {
            println!("Skipping real synthesis: {}", probe.note_vi);
            return;
        }

        let tmp = tempdir().unwrap();
        let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();
        let out_dir = ws.cache_dir().join("turns").join("test-turn");
        fs::create_dir_all(&out_dir).unwrap();

        let input = json!({
            "text": "Xin chào thế giới",
            "voice": "Bình",
            "speed": 0.9
        });

        let rt = tokio::runtime::Runtime::new().unwrap();
        let res = rt.block_on(run_vieneu(&ws, &out_dir, &input));
        assert!(res.is_ok(), "Real synthesis failed: {:?}", res);
    }

    #[test]
    #[ignore]
    fn test_v03_evidence_real_vieneu_voices() {
        let probe = probe_vieneu();
        if !probe.available {
            println!("Skipping real synthesis: {}", probe.note_vi);
            return;
        }

        let evidence_dir = PathBuf::from(
            "D:/Users/Dang/LocalContentStudio/.aw/tasks/2026-10-24-v03-toc-do-giong/evidence",
        );
        let _ = fs::create_dir_all(&evidence_dir);

        let sample_text = "Hôm nay, ngày 24/10, tại TP.HCM, nhiệt độ đạt 32°C. Giá xăng tăng 1.500 đ, tương đương 6,5%.\n\n[ngắt 0.7] Bạn đã chuẩn bị cho kế hoạch lúc 15h30 chưa? Hãy cùng đón xem các tin tức mới nhất về CNTT và AI nhé!";

        let tmp = tempdir().unwrap();
        let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();
        let out_dir = ws.cache_dir().join("turns").join("test-vieneu-evidence");
        fs::create_dir_all(&out_dir).unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();

        let voices = [
            ("Bình", "vieneu_binh.wav"),
            ("Mai", "vieneu_mai.wav"),
            ("Nam", "vieneu_nam.wav"),
            ("Nữ", "vieneu_nu.wav"),
        ];

        println!("=== V03 VIENEU MULTI-VOICE BENCHMARK RESULTS ===");

        for (voice, out_name) in voices {
            let input = json!({
                "text": sample_text,
                "voice": voice,
            });

            let start = std::time::Instant::now();
            let res = rt
                .block_on(run_vieneu(&ws, &out_dir, &input))
                .expect("run_vieneu failed");
            let elapsed_s = start.elapsed().as_secs_f64();

            let dur_s = res["duration_seconds"].as_f64().unwrap();
            let sps = res["syllables_per_second"].as_f64().unwrap();
            let eff_speed = res["effective_speed"].as_f64().unwrap();
            let actual_voice = res["voice"].as_str().unwrap_or(voice);

            let gen_wav = ws.root().join(res["audio_path"].as_str().unwrap());
            let target_ev = evidence_dir.join(out_name);
            let _ = fs::copy(&gen_wav, &target_ev);

            let ratio = elapsed_s / dur_s;
            println!(
                "Voice '{}' (mapped: '{}'): synth_time = {:.2}s, audio_duration = {:.2}s, ratio = {:.2}x, sps = {:.2}, effective_speed = {:.2}",
                voice, actual_voice, elapsed_s, dur_s, ratio, sps, eff_speed
            );
        }
    }
}
