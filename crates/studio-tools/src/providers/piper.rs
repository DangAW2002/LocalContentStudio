//! Piper TTS provider (fast, offline neural TTS using ONNX runtime).
//!
//! Models are downloaded only after explicit confirmation into:
//! `%LOCALAPPDATA%/LocalContentStudio/models/piper/<voice>/` with SHA-256 verification.
//! Supported Vietnamese voices from pinned Hugging Face revision v1.0.0:
//! - `vi_VN-vais1000-medium` (default)
//! - `vi_VN-25hours_single-low`
//! - `vi_VN-vivos-x_low`
//!
//! Piper binary is located on PATH or installed into `%LOCALAPPDATA%/LocalContentStudio/bin/piper/`.

use crate::error::{Result, ToolError};
use crate::probe::{find_in_path, ProbeResult};
use hex::ToHex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use studio_workspace::Workspace;

/// Pinned Piper binary release version and download specification.
pub const PIPER_RELEASE_VERSION: &str = "2023.11.14-2";
pub const PIPER_ZIP_URL: &str =
    "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_windows_amd64.zip";
pub const PIPER_ZIP_SHA256: &str =
    "f3c58906402b24f3a96d92145f58acba6d86c9b5db896d207f78dc80811efcea";
pub const PIPER_ZIP_SIZE_BYTES: u64 = 22_477_236;

/// Pinned Hugging Face revision tag for Piper voices.
pub const PIPER_HF_REVISION: &str = "v1.0.0";
pub const PIPER_HF_BASE_URL: &str = "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0";

/// License warning note required for all Vietnamese fine-tuned Piper voices.
pub const VIETNAMESE_VOICE_LICENSE_NOTE: &str =
    "Dữ liệu giọng đọc CC-BY 4.0 nhưng fine-tune từ mô hình gốc dành cho nghiên cứu → chỉ nên dùng cá nhân/phi thương mại.";

/// Specification of a Piper Vietnamese voice.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PiperVoiceMeta {
    pub id: &'static str,
    pub name: &'static str,
    pub quality: &'static str,
    pub num_speakers: u32,
    pub onnx_relative_path: &'static str,
    pub onnx_sha256: &'static str,
    pub onnx_size_bytes: u64,
    pub json_relative_path: &'static str,
    pub json_sha256: &'static str,
    pub json_size_bytes: u64,
}

/// Catalog of supported Vietnamese voices for Piper TTS.
pub const PIPER_VIETNAMESE_VOICES: &[PiperVoiceMeta] = &[
    PiperVoiceMeta {
        id: "vi_VN-vais1000-medium",
        name: "vais1000 (Trung bình - 22kHz)",
        quality: "medium",
        num_speakers: 1,
        onnx_relative_path: "vi/vi_VN/vais1000/medium/vi_VN-vais1000-medium.onnx",
        onnx_sha256: "ec7c89e2c85f4d1edc24b6120c18aaf1bda614f06b511567eb9c7c0de15e2dab",
        onnx_size_bytes: 63_201_294,
        json_relative_path: "vi/vi_VN/vais1000/medium/vi_VN-vais1000-medium.onnx.json",
        json_sha256: "fafb9da1354ed4b77c31af228ed41fb41cd825c14cffa105454b25e6ae751ee0",
        json_size_bytes: 4_860,
    },
    PiperVoiceMeta {
        id: "vi_VN-25hours_single-low",
        name: "25hours_single (Nhẹ - 16kHz)",
        quality: "low",
        num_speakers: 1,
        onnx_relative_path: "vi/vi_VN/25hours_single/low/vi_VN-25hours_single-low.onnx",
        onnx_sha256: "266945c4a80cb3301450c8ba6fcf6ba3542c56cf952a08a8ff3d2d4ff0ab5609",
        onnx_size_bytes: 63_104_526,
        json_relative_path: "vi/vi_VN/25hours_single/low/vi_VN-25hours_single-low.onnx.json",
        json_sha256: "344b5849f5bc585031a63b9cc75f96e2c469f3d98879d8d3acbc2fce6d470064",
        json_size_bytes: 4_176,
    },
    PiperVoiceMeta {
        id: "vi_VN-vivos-x_low",
        name: "vivos (Siêu nhẹ - 16kHz, 65 giọng)",
        quality: "x_low",
        num_speakers: 65,
        onnx_relative_path: "vi/vi_VN/vivos/x_low/vi_VN-vivos-x_low.onnx",
        onnx_sha256: "6ab13374eb0862021a545befe7727aef59e16117f1c075aa9e0362237ecc98ae",
        onnx_size_bytes: 27_789_413,
        json_relative_path: "vi/vi_VN/vivos/x_low/vi_VN-vivos-x_low.onnx.json",
        json_sha256: "9dc373d69b0e4f39d864cb8ac9fe42cee49817a0f02d831e57a9ce6e944e234a",
        json_size_bytes: 5_592,
    },
];

/// Returns the base directory for Piper models:
/// `%LOCALAPPDATA%/LocalContentStudio/models/piper/`
pub fn piper_models_dir() -> PathBuf {
    let base = if let Ok(local) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(local).join("LocalContentStudio")
    } else if let Ok(app_data) = std::env::var("APPDATA") {
        PathBuf::from(app_data).join("LocalContentStudio")
    } else if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        PathBuf::from(home).join(".localcontentstudio")
    } else {
        PathBuf::from(".studio-local-data")
    };
    base.join("models").join("piper")
}

/// Returns the base directory for Piper binary:
/// `%LOCALAPPDATA%/LocalContentStudio/bin/piper/`
pub fn piper_binary_dir() -> PathBuf {
    let base = if let Ok(local) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(local).join("LocalContentStudio")
    } else if let Ok(app_data) = std::env::var("APPDATA") {
        PathBuf::from(app_data).join("LocalContentStudio")
    } else if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        PathBuf::from(home).join(".localcontentstudio")
    } else {
        PathBuf::from(".studio-local-data")
    };
    base.join("bin").join("piper")
}

/// Locate Piper binary: checks PATH first, then standard local app locations.
pub fn find_piper_binary() -> Option<PathBuf> {
    if let Some(path) = find_in_path("piper") {
        return Some(path);
    }

    let local_bin = piper_binary_dir();
    let candidates = [
        local_bin.join("piper.exe"),
        local_bin.join("piper").join("piper.exe"),
        local_bin.join("piper"),
    ];

    for candidate in &candidates {
        if candidate.is_file() {
            return Some(candidate.clone());
        }
    }

    // Also check app-level tools directory
    let app_paths = studio_workspace::AppPaths::from_env();
    let app_tool = app_paths
        .data_dir()
        .join("tools")
        .join("piper")
        .join("piper.exe");
    if app_tool.is_file() {
        return Some(app_tool);
    }

    None
}

/// Check which Vietnamese voices have been downloaded and are valid on disk.
pub fn get_installed_piper_voices() -> Vec<String> {
    let root = piper_models_dir();
    let mut installed = Vec::new();

    for voice in PIPER_VIETNAMESE_VOICES {
        let v_dir = root.join(voice.id);
        let onnx_file = v_dir.join(format!("{}.onnx", voice.id));
        let json_file = v_dir.join(format!("{}.onnx.json", voice.id));

        if onnx_file.is_file() && json_file.is_file() {
            installed.push(voice.id.to_string());
        }
    }

    installed
}

/// Probe Piper status without faking capabilities.
pub fn probe_piper() -> ProbeResult {
    let mut missing = Vec::new();

    let bin_found = find_piper_binary();
    if bin_found.is_none() {
        missing.push("bin:piper".to_string());
    }

    let installed_voices = get_installed_piper_voices();
    if installed_voices.is_empty() {
        missing.push("model:piper".to_string());
    }

    if missing.is_empty() {
        ProbeResult {
            available: true,
            missing: Vec::new(),
            note_vi: format!(
                "Sẵn sàng (Piper TTS, {} giọng: {})",
                installed_voices.len(),
                installed_voices.join(", ")
            ),
        }
    } else {
        let note = if bin_found.is_none() && installed_voices.is_empty() {
            "Chưa cài đặt Piper TTS và chưa tải mô hình giọng đọc. Vui lòng cài đặt trong Cài đặt → Giọng đọc (TTS).".to_string()
        } else if bin_found.is_none() {
            "Chưa cài đặt binary Piper. Vui lòng tải binary trong Cài đặt → Giọng đọc (TTS)."
                .to_string()
        } else {
            "Đã có Piper nhưng chưa tải mô hình giọng đọc tiếng Việt nào. Vui lòng tải trong Cài đặt → Giọng đọc (TTS).".to_string()
        };

        ProbeResult {
            available: false,
            missing,
            note_vi: note,
        }
    }
}

/// Verify a file's SHA-256 against an expected hex string (case-insensitive).
pub fn verify_sha256(path: &Path, expected_hex: &str) -> Result<bool> {
    let mut file = fs::File::open(path).map_err(|e| ToolError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let bytes_read = file.read(&mut buffer).map_err(|e| ToolError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let calculated = hasher.finalize().encode_hex::<String>();
    Ok(calculated.eq_ignore_ascii_case(expected_hex))
}

/// Options for customizing Piper speech synthesis CLI arguments.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PiperCliOptions {
    /// Speech speed multiplier (e.g. 0.9).
    pub speed: Option<f32>,
    /// Speaker ID for multi-speaker models.
    pub speaker: Option<u32>,
    /// Generator noise scale (default 0.667).
    pub noise_scale: Option<f32>,
    /// Phoneme width noise scale (default 0.85).
    pub noise_w: Option<f32>,
    /// Silence after each sentence in seconds.
    pub sentence_silence: Option<f32>,
}

/// Construct CLI arguments for invoking the Piper binary with comprehensive parameters.
pub fn build_piper_args_full(
    model_path: &Path,
    config_path: &Path,
    output_wav_path: &Path,
    options: &PiperCliOptions,
) -> Vec<String> {
    let mut args = vec![
        "--model".to_string(),
        model_path.to_string_lossy().to_string(),
        "--config".to_string(),
        config_path.to_string_lossy().to_string(),
        "--output_file".to_string(),
        output_wav_path.to_string_lossy().to_string(),
    ];

    if let Some(s) = options.speed {
        if (s - 1.0).abs() > f32::EPSILON && s > 0.1 && s < 5.0 {
            // Piper uses --length_scale where higher value means slower speech (duration scale)
            let length_scale = 1.0 / s;
            args.push("--length_scale".to_string());
            args.push(format!("{length_scale:.2}"));
        }
    }

    if let Some(spk) = options.speaker {
        args.push("--speaker".to_string());
        args.push(spk.to_string());
    }

    if let Some(ns) = options.noise_scale {
        args.push("--noise_scale".to_string());
        args.push(format!("{ns:.3}"));
    }

    if let Some(nw) = options.noise_w {
        args.push("--noise_w".to_string());
        args.push(format!("{nw:.3}"));
    }

    if let Some(ss) = options.sentence_silence {
        args.push("--sentence_silence".to_string());
        args.push(format!("{ss:.3}"));
    }

    args
}

/// Construct CLI arguments for invoking the Piper binary (backward-compatible signature).
pub fn build_piper_args(
    model_path: &Path,
    config_path: &Path,
    output_wav_path: &Path,
    speed: Option<f32>,
    speaker: Option<u32>,
) -> Vec<String> {
    build_piper_args_full(
        model_path,
        config_path,
        output_wav_path,
        &PiperCliOptions {
            speed,
            speaker,
            ..Default::default()
        },
    )
}

/// Run Piper speech synthesis in-process via binary with natural punctuation pausing,
/// Vietnamese text normalization, noise modulation, and deterministic cache.
pub async fn run_piper(ws: &Workspace, out_dir: &Path, input: &Value) -> Result<Value> {
    let piper_exe = find_piper_binary().ok_or_else(|| ToolError::MissingDependency {
        tool_id: "tts.piper".into(),
        missing: vec![
            "bin:piper (Chưa cài đặt Piper TTS. Vui lòng tải trong Cài đặt → Giọng đọc (TTS))"
                .into(),
        ],
    })?;

    let text = input
        .get("text")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| {
            ToolError::ParseError("Thiếu tham số 'text' để tổng hợp giọng nói".into())
        })?;

    let installed = get_installed_piper_voices();
    if installed.is_empty() {
        return Err(ToolError::ExecutionFailed {
            tool_id: "tts.piper".into(),
            reason: "Chưa có mô hình giọng đọc tiếng Việt nào được tải về. Vui lòng tải trong Cài đặt → Giọng đọc (TTS).".into(),
        });
    }

    // Voice selection: requested voice, or default vi_VN-vais1000-medium, or first installed
    let voice_req = input.get("voice").and_then(Value::as_str);
    let chosen_voice_id = if let Some(v) = voice_req {
        if installed.iter().any(|iv| iv == v) {
            v.to_string()
        } else {
            installed[0].clone()
        }
    } else if installed.contains(&"vi_VN-vais1000-medium".to_string()) {
        "vi_VN-vais1000-medium".to_string()
    } else {
        installed[0].clone()
    };

    let models_root = piper_models_dir();
    let voice_dir = models_root.join(&chosen_voice_id);
    let model_onnx = voice_dir.join(format!("{chosen_voice_id}.onnx"));
    let model_json = voice_dir.join(format!("{chosen_voice_id}.onnx.json"));

    if !model_onnx.is_file() || !model_json.is_file() {
        return Err(ToolError::ExecutionFailed {
            tool_id: "tts.piper".into(),
            reason: format!(
                "Mô hình giọng đọc '{chosen_voice_id}' chưa hoàn thiện (thiếu tệp onnx hoặc json)."
            ),
        });
    }

    // Default speed for Vietnamese is 0.9 if not specified
    let speed_req = input
        .get("speed")
        .and_then(Value::as_f64)
        .map(|s| s as f32)
        .unwrap_or(0.9);

    // Default noise modulation parameters for natural prosody
    let noise_scale_req = input
        .get("noise_scale")
        .and_then(Value::as_f64)
        .map(|s| s as f32)
        .unwrap_or(0.667);

    let noise_w_req = input
        .get("noise_w")
        .and_then(Value::as_f64)
        .map(|s| s as f32)
        .unwrap_or(0.85);

    let speaker_req = input
        .get("speaker")
        .and_then(Value::as_u64)
        .map(|s| s as u32);

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
            let path = out_dir.join(format!("piper-{file_id}.wav"));
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            path
        };

    let available_ids: Vec<&str> = PIPER_VIETNAMESE_VOICES.iter().map(|v| v.id).collect();

    // Check TTS audio cache by input hash
    let cache_key = crate::tts::compute_tts_cache_key("tts.piper", input);
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

        return Ok(json!({
            "audio_path": rel_path,
            "duration_seconds": (duration_s * 1000.0).round() / 1000.0,
            "provider": "tts.piper",
            "voice": chosen_voice_id,
            "speed": speed_req,
            "noise_scale": noise_scale_req,
            "noise_w": noise_w_req,
            "available_voices": available_ids,
            "cached": true,
            "note": format!("Tổng hợp giọng nói thành công qua Piper TTS (bản lưu cache {chosen_voice_id}). {VIETNAMESE_VOICE_LICENSE_NOTE}")
        }));
    }

    // Step 1: Normalize Vietnamese text
    let normalized = crate::tts::normalize_vietnamese_text(text);

    // Step 2: Split into speech segments by punctuation and tags
    let pauses = crate::tts::PauseConfig::from_value(input.get("pauses"));
    let segments = crate::tts::split_into_tts_segments(&normalized, &pauses);

    if segments.is_empty() {
        return Err(ToolError::ExecutionFailed {
            tool_id: "tts.piper".into(),
            reason: "Nội dung văn bản sau chuẩn hoá rỗng.".into(),
        });
    }

    let chunks_dir = ws
        .cache_dir()
        .join("tts_chunks")
        .join(uuid::Uuid::new_v4().to_string());
    fs::create_dir_all(&chunks_dir).map_err(|e| ToolError::Io {
        path: chunks_dir.clone(),
        source: e,
    })?;

    let mut chunk_items: Vec<(PathBuf, f32)> = Vec::new();

    for (idx, seg) in segments.iter().enumerate() {
        let chunk_wav = chunks_dir.join(format!("chunk_{idx}.wav"));
        let cmd_args = build_piper_args_full(
            &model_onnx,
            &model_json,
            &chunk_wav,
            &PiperCliOptions {
                speed: Some(speed_req),
                speaker: speaker_req,
                noise_scale: Some(noise_scale_req),
                noise_w: Some(noise_w_req),
                sentence_silence: Some(0.0), // Disable internal sentence padding in favor of exact WAV silence
            },
        );

        let mut child = Command::new(&piper_exe)
            .args(&cmd_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(ws.root())
            .spawn()
            .map_err(|e| ToolError::ExecutionFailed {
                tool_id: "tts.piper".into(),
                reason: format!("Không thể khởi chạy Piper binary: {e}"),
            })?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(seg.text.as_bytes())
                .map_err(|e| ToolError::Io {
                    path: PathBuf::from("stdin"),
                    source: e,
                })?;
            let _ = stdin.flush();
        }

        let output = child
            .wait_with_output()
            .map_err(|e| ToolError::ExecutionFailed {
                tool_id: "tts.piper".into(),
                reason: format!("Lỗi chờ tiến trình Piper hoàn thành: {e}"),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let _ = fs::remove_dir_all(&chunks_dir);
            return Err(ToolError::ExecutionFailed {
                tool_id: "tts.piper".into(),
                reason: format!(
                    "Tiến trình Piper kết thúc với lỗi {}: {}",
                    output.status,
                    stderr.trim()
                ),
            });
        }

        if !chunk_wav.is_file() {
            let _ = fs::remove_dir_all(&chunks_dir);
            return Err(ToolError::ExecutionFailed {
                tool_id: "tts.piper".into(),
                reason: format!(
                    "Piper kết thúc nhưng không tìm thấy tệp âm thanh phân đoạn #{idx}."
                ),
            });
        }

        chunk_items.push((chunk_wav, seg.pause_after_s));
    }

    // Step 3: Concatenate all segments with deterministic silence gaps
    let item_refs: Vec<(&Path, f32)> = chunk_items.iter().map(|(p, s)| (p.as_path(), *s)).collect();
    let total_duration = crate::tts::concat_wav_files_with_silence(&item_refs, &target_wav_path)?;

    // Clean up temporary chunks
    let _ = fs::remove_dir_all(&chunks_dir);

    // Step 4: Write to cache
    let _ = fs::create_dir_all(&cache_dir);
    let _ = fs::copy(&target_wav_path, &cached_file);

    let rel_path = target_wav_path
        .strip_prefix(ws.root())
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| target_wav_path.to_string_lossy().replace('\\', "/"));

    Ok(json!({
        "audio_path": rel_path,
        "duration_seconds": total_duration,
        "provider": "tts.piper",
        "voice": chosen_voice_id,
        "speed": speed_req,
        "noise_scale": noise_scale_req,
        "noise_w": noise_w_req,
        "available_voices": available_ids,
        "cached": false,
        "note": format!("Tổng hợp giọng nói thành công qua Piper TTS ({chosen_voice_id}). {VIETNAMESE_VOICE_LICENSE_NOTE}")
    }))
}

/// Download a file from URL and verify its SHA-256 before saving to target_path.
pub async fn download_with_sha256(
    url: &str,
    expected_sha256: &str,
    target_path: &Path,
) -> Result<()> {
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| ToolError::Io {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| ToolError::ExecutionFailed {
            tool_id: "tts.piper".into(),
            reason: format!("Không thể khởi tạo HTTP client: {e}"),
        })?;

    let resp = client
        .get(url)
        .header("User-Agent", "LocalContentStudio/1.0")
        .send()
        .await
        .map_err(|e| ToolError::ExecutionFailed {
            tool_id: "tts.piper".into(),
            reason: format!("Lỗi tải dữ liệu từ '{url}': {e}"),
        })?;

    if !resp.status().is_success() {
        return Err(ToolError::ExecutionFailed {
            tool_id: "tts.piper".into(),
            reason: format!("Máy chủ từ chối yêu cầu (HTTP {}): {url}", resp.status()),
        });
    }

    let bytes = resp.bytes().await.map_err(|e| ToolError::ExecutionFailed {
        tool_id: "tts.piper".into(),
        reason: format!("Lỗi đọc nội dung phản hồi HTTP: {e}"),
    })?;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let calculated = hasher.finalize().encode_hex::<String>();

    if !calculated.eq_ignore_ascii_case(expected_sha256) {
        return Err(ToolError::ExecutionFailed {
            tool_id: "tts.piper".into(),
            reason: format!(
                "Mã băm SHA-256 không khớp! Dự kiến: {expected_sha256}, tính toán: {calculated}."
            ),
        });
    }

    fs::write(target_path, &bytes).map_err(|e| ToolError::Io {
        path: target_path.to_path_buf(),
        source: e,
    })?;

    Ok(())
}

/// Download and install Piper binary from official GitHub release with SHA-256 check.
pub async fn install_piper_binary() -> Result<PathBuf> {
    let bin_dir = piper_binary_dir();
    fs::create_dir_all(&bin_dir).map_err(|e| ToolError::Io {
        path: bin_dir.clone(),
        source: e,
    })?;

    let temp_zip_path = bin_dir.join("piper_temp.zip");
    download_with_sha256(PIPER_ZIP_URL, PIPER_ZIP_SHA256, &temp_zip_path).await?;

    // Extract zip file safely
    let file = fs::File::open(&temp_zip_path).map_err(|e| ToolError::Io {
        path: temp_zip_path.clone(),
        source: e,
    })?;

    let mut archive = zip::ZipArchive::new(file).map_err(|e| ToolError::ExecutionFailed {
        tool_id: "tts.piper".into(),
        reason: format!("Không thể giải nén tệp zip Piper: {e}"),
    })?;

    for i in 0..archive.len() {
        let mut zip_file = archive
            .by_index(i)
            .map_err(|e| ToolError::ExecutionFailed {
                tool_id: "tts.piper".into(),
                reason: format!("Lỗi đọc mục zip: {e}"),
            })?;

        let raw_name = zip_file.name().to_string();
        // Prevent zip-slip
        if raw_name.contains("..") || raw_name.starts_with('/') || raw_name.starts_with('\\') {
            continue;
        }

        let out_path = bin_dir.join(raw_name);
        if zip_file.is_dir() {
            let _ = fs::create_dir_all(&out_path);
        } else {
            if let Some(p) = out_path.parent() {
                let _ = fs::create_dir_all(p);
            }
            let mut outfile = fs::File::create(&out_path).map_err(|e| ToolError::Io {
                path: out_path.clone(),
                source: e,
            })?;
            std::io::copy(&mut zip_file, &mut outfile).map_err(|e| ToolError::Io {
                path: out_path.clone(),
                source: e,
            })?;
        }
    }

    let _ = fs::remove_file(&temp_zip_path);

    find_piper_binary().ok_or_else(|| ToolError::ExecutionFailed {
        tool_id: "tts.piper".into(),
        reason: "Đã giải nén Piper nhưng không tìm thấy tệp piper.exe.".into(),
    })
}

/// Download and install a specific Vietnamese voice model into local storage with SHA-256 check.
pub async fn install_piper_voice(voice_id: &str) -> Result<()> {
    let meta = PIPER_VIETNAMESE_VOICES
        .iter()
        .find(|v| v.id == voice_id)
        .ok_or_else(|| ToolError::ToolNotFound(format!("Không hỗ trợ giọng đọc: {voice_id}")))?;

    let target_dir = piper_models_dir().join(meta.id);
    fs::create_dir_all(&target_dir).map_err(|e| ToolError::Io {
        path: target_dir.clone(),
        source: e,
    })?;

    let onnx_url = format!("{}/{}", PIPER_HF_BASE_URL, meta.onnx_relative_path);
    let json_url = format!("{}/{}", PIPER_HF_BASE_URL, meta.json_relative_path);

    let target_onnx = target_dir.join(format!("{}.onnx", meta.id));
    let target_json = target_dir.join(format!("{}.onnx.json", meta.id));

    // Download ONNX
    download_with_sha256(&onnx_url, meta.onnx_sha256, &target_onnx).await?;

    // Download JSON config
    download_with_sha256(&json_url, meta.json_sha256, &target_json).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_build_piper_args_options() {
        let model = Path::new("C:/models/vi_VN-vais1000-medium.onnx");
        let config = Path::new("C:/models/vi_VN-vais1000-medium.onnx.json");
        let out = Path::new("C:/out/speech.wav");

        let args = build_piper_args(model, config, out, Some(1.25), Some(3));

        assert_eq!(args[0], "--model");
        assert_eq!(args[1], model.to_string_lossy());
        assert_eq!(args[2], "--config");
        assert_eq!(args[3], config.to_string_lossy());
        assert_eq!(args[4], "--output_file");
        assert_eq!(args[5], out.to_string_lossy());

        assert!(args.contains(&"--length_scale".to_string()));
        let l_idx = args.iter().position(|a| a == "--length_scale").unwrap();
        // 1.0 / 1.25 = 0.80
        assert_eq!(args[l_idx + 1], "0.80");

        assert!(args.contains(&"--speaker".to_string()));
        let s_idx = args.iter().position(|a| a == "--speaker").unwrap();
        assert_eq!(args[s_idx + 1], "3");
    }

    #[test]
    fn test_build_piper_args_full_options() {
        let model = Path::new("C:/models/vi_VN-vais1000-medium.onnx");
        let config = Path::new("C:/models/vi_VN-vais1000-medium.onnx.json");
        let out = Path::new("C:/out/speech.wav");

        let args = build_piper_args_full(
            model,
            config,
            out,
            &PiperCliOptions {
                speed: Some(0.9),
                speaker: Some(2),
                noise_scale: Some(0.667),
                noise_w: Some(0.85),
                sentence_silence: Some(0.0),
            },
        );

        assert!(args.contains(&"--noise_scale".to_string()));
        let ns_idx = args.iter().position(|a| a == "--noise_scale").unwrap();
        assert_eq!(args[ns_idx + 1], "0.667");

        assert!(args.contains(&"--noise_w".to_string()));
        let nw_idx = args.iter().position(|a| a == "--noise_w").unwrap();
        assert_eq!(args[nw_idx + 1], "0.850");

        assert!(args.contains(&"--sentence_silence".to_string()));
        let ss_idx = args.iter().position(|a| a == "--sentence_silence").unwrap();
        assert_eq!(args[ss_idx + 1], "0.000");
    }

    #[test]
    fn test_piper_voice_metadata_constants() {
        assert_eq!(PIPER_VIETNAMESE_VOICES.len(), 3);
        assert_eq!(PIPER_VIETNAMESE_VOICES[0].id, "vi_VN-vais1000-medium");
        assert_eq!(PIPER_VIETNAMESE_VOICES[1].id, "vi_VN-25hours_single-low");
        assert_eq!(PIPER_VIETNAMESE_VOICES[2].id, "vi_VN-vivos-x_low");

        for voice in PIPER_VIETNAMESE_VOICES {
            assert_eq!(voice.onnx_sha256.len(), 64);
            assert_eq!(voice.json_sha256.len(), 64);
            assert!(voice.onnx_size_bytes > 0);
            assert!(voice.json_size_bytes > 0);
        }
    }

    #[test]
    fn test_sha256_verification_logic() {
        let tmp = tempdir().unwrap();
        let file_path = tmp.path().join("test.bin");
        let content = b"piper-offline-tts-vietnamese";
        fs::write(&file_path, content).unwrap();

        let mut hasher = Sha256::new();
        hasher.update(content);
        let expected = hasher.finalize().encode_hex::<String>();

        let valid = verify_sha256(&file_path, &expected).unwrap();
        assert!(valid, "SHA-256 verification should match");

        let invalid = verify_sha256(
            &file_path,
            "0000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap();
        assert!(!invalid, "SHA-256 mismatch should be detected");
    }

    #[test]
    #[ignore]
    fn test_real_piper_synthesis() {
        let bin = find_piper_binary();
        if bin.is_none() {
            println!("Skipping real synthesis: Piper binary not found");
            return;
        }

        let installed = get_installed_piper_voices();
        if installed.is_empty() {
            println!("Skipping real synthesis: No Piper voices installed");
            return;
        }

        let tmp = tempdir().unwrap();
        let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();
        let out_dir = ws.cache_dir().join("turns").join("test-piper");
        fs::create_dir_all(&out_dir).unwrap();

        let input = json!({
            "text": "Chào bạn, đây là thử nghiệm giọng đọc Piper offline. Ngày 24/10/2026, chúng tôi đạt 1.500 lượt xem, tăng 25%. Bạn có thấy tự nhiên không?",
            "voice": installed[0],
            "speed": 0.9
        });

        let rt = tokio::runtime::Runtime::new().unwrap();
        let res = rt.block_on(run_piper(&ws, &out_dir, &input));
        assert!(res.is_ok(), "Real Piper synthesis failed: {:?}", res);
        let val = res.unwrap();
        assert!(val.get("duration_seconds").is_some());
        assert!(val["duration_seconds"].as_f64().unwrap() > 0.0);
    }

    #[test]
    #[ignore]
    fn test_v01_evidence_real_synthesis() {
        let bin = find_piper_binary().expect("Piper binary required for real synthesis test");
        let installed = get_installed_piper_voices();
        assert!(!installed.is_empty(), "Piper voices required");

        let voice_id = &installed[0];
        let models_root = piper_models_dir();
        let voice_dir = models_root.join(voice_id);
        let model_onnx = voice_dir.join(format!("{voice_id}.onnx"));
        let model_json = voice_dir.join(format!("{voice_id}.onnx.json"));

        let evidence_dir = PathBuf::from(
            "D:/Users/Dang/LocalContentStudio/.aw/tasks/2026-10-24-v01-giong-doc-tu-nhien/evidence",
        );
        let _ = fs::create_dir_all(&evidence_dir);

        let sample_text = "Chào mừng quý vị đến với bản tin hôm nay, ngày 24/10. Tại TP.HCM, hơn 1.500 doanh nghiệp vừa ghi nhận mức tăng trưởng doanh thu ấn tượng 25%, cao gấp 2,5 lần so với cùng kỳ.\n\n[ngắt 0.7] Bạn đã nắm bắt cơ hội này chưa? Đừng bỏ lỡ các thông tin cập nhật mới nhất từ chúng tôi, v.v.";

        // --- 1. Synthesize BEFORE (old behavior: no pause splitting, no normalization, speed 1.0) ---
        let before_wav = evidence_dir.join("before.wav");
        let legacy_text = sample_text.replace("[ngắt 0.7]", "");
        let before_args = build_piper_args_full(
            &model_onnx,
            &model_json,
            &before_wav,
            &PiperCliOptions {
                speed: Some(1.0),
                speaker: None,
                noise_scale: Some(0.667),
                noise_w: Some(0.8),
                sentence_silence: Some(0.2),
            },
        );

        let mut child_before = Command::new(&bin)
            .args(&before_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn before piper");

        if let Some(mut stdin) = child_before.stdin.take() {
            stdin.write_all(legacy_text.as_bytes()).unwrap();
            let _ = stdin.flush();
        }
        let out_before = child_before.wait_with_output().expect("wait before piper");
        assert!(out_before.status.success(), "before synthesis must succeed");

        let before_duration = if let Ok((spec, data)) = crate::tts::read_pcm_wav(&before_wav) {
            spec.duration_seconds(data.len())
        } else {
            0.0
        };

        // --- 2. Synthesize AFTER (new behavior: norm, punctuation pause splitting, speed 0.9, noise_scale 0.667, noise_w 0.85) ---
        let after_wav = evidence_dir.join("after.wav");

        let tmp = tempdir().unwrap();
        let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();
        let out_dir = ws.cache_dir().join("turns").join("test-evidence");
        fs::create_dir_all(&out_dir).unwrap();

        let after_input = json!({
            "text": sample_text,
            "voice": voice_id,
            "speed": 0.9,
            "noise_scale": 0.667,
            "noise_w": 0.85,
        });

        let rt = tokio::runtime::Runtime::new().unwrap();
        let res = rt.block_on(run_piper(&ws, &out_dir, &after_input));
        assert!(res.is_ok(), "after synthesis must succeed: {:?}", res);

        let after_val = res.unwrap();
        let rel_audio = after_val["audio_path"].as_str().unwrap();
        let generated_wav = ws.root().join(rel_audio);
        fs::copy(&generated_wav, &after_wav).expect("copy to after.wav");

        let after_duration = after_val["duration_seconds"].as_f64().unwrap();

        let word_count = 59.0;
        let before_wps = word_count / before_duration;
        let after_wps = word_count / after_duration;

        println!("=== V01 EVIDENCE TEST RESULTS ===");
        println!("Sample text (~60 words): 59 words");
        println!(
            "Before duration: {:.3}s -> {:.2} words/s",
            before_duration, before_wps
        );
        println!(
            "After duration:  {:.3}s -> {:.2} words/s",
            after_duration, after_wps
        );
        println!("Before file: {}", before_wav.display());
        println!("After file:  {}", after_wav.display());
    }
}
