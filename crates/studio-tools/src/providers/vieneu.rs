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

/// Available default preset voices for VieNeu-TTS.
pub const VIENEU_DEFAULT_VOICES: &[&str] = &["Bình", "Mai", "Nam", "Nữ"];

/// Options for configuring VieNeu-TTS synthesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VieneuVoiceConfig {
    /// Voice preset name (e.g., "Bình", "Mai", "Nam", "Nữ").
    pub voice: String,
    /// Speed multiplier (default 1.0).
    pub speed: f32,
    /// Optional workspace-relative reference audio file for voice cloning.
    pub reference_audio: Option<String>,
}

impl Default for VieneuVoiceConfig {
    fn default() -> Self {
        Self {
            voice: "Bình".to_string(),
            speed: 1.0,
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
    // Write or refresh helper script
    fs::write(&script_path, VIENEU_HELPER_SCRIPT).map_err(|e| ToolError::Io {
        path: script_path.clone(),
        source: e,
    })?;

    Ok(script_path)
}

/// Run VieNeu-TTS synthesis in-process via Python helper.
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
    let speed_req = input.get("speed").and_then(Value::as_f64).map(|s| s as f32);

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

    let py_exe = find_in_path("python")
        .or_else(|| find_in_path("python3"))
        .ok_or_else(|| ToolError::MissingDependency {
            tool_id: "tts.vieneu".into(),
            missing: vec!["bin:python".into()],
        })?;

    let script_path = ensure_helper_script(&ws.cache_dir())?;

    // Create a temporary file for the text (UTF-8, avoiding shell quoting)
    let mut temp_txt = tempfile::Builder::new()
        .prefix("vieneu-input-")
        .suffix(".txt")
        .tempfile()
        .map_err(|e| ToolError::Io {
            path: PathBuf::from("tempfile"),
            source: e,
        })?;

    temp_txt
        .write_all(text.as_bytes())
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
        &target_wav_path,
        voice_req,
        speed_req,
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

    if !target_wav_path.is_file() {
        return Err(ToolError::ExecutionFailed {
            tool_id: "tts.vieneu".into(),
            reason: "VieNeu-TTS kết thúc nhưng không tạo được tệp âm thanh đầu ra.".into(),
        });
    }

    let rel_path = target_wav_path
        .strip_prefix(ws.root())
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| target_wav_path.to_string_lossy().replace('\\', "/"));

    let actual_voice = voice_req.unwrap_or("Bình").to_string();

    Ok(json!({
        "audio_path": rel_path,
        "provider": "tts.vieneu",
        "voice": actual_voice,
        "available_voices": VIENEU_DEFAULT_VOICES,
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
            "speed": 1.0
        });

        let rt = tokio::runtime::Runtime::new().unwrap();
        let res = rt.block_on(run_vieneu(&ws, &out_dir, &input));
        assert!(res.is_ok(), "Real synthesis failed: {:?}", res);
    }
}
