//! WAV PCM file reader, writer, and deterministic concatenation with silence gaps.
//!
//! Provides sample-accurate PCM audio concatenation in pure Rust without external process overhead.

use crate::error::{Result, ToolError};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// Specification of a PCM WAV file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WavSpec {
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub byte_rate: u32,
    pub block_align: u16,
}

impl WavSpec {
    /// Calculate the duration in seconds for a given data byte length.
    pub fn duration_seconds(&self, data_len_bytes: usize) -> f64 {
        if self.byte_rate == 0 {
            0.0
        } else {
            data_len_bytes as f64 / self.byte_rate as f64
        }
    }

    /// Compute byte count for a given silence duration in seconds.
    pub fn silence_bytes(&self, duration_s: f32) -> usize {
        if duration_s <= 0.0 {
            return 0;
        }
        let samples = (self.sample_rate as f64 * duration_s as f64).round() as usize;
        samples * self.block_align as usize
    }
}

/// Read PCM WAV file and extract its specification and raw audio sample bytes.
pub fn read_pcm_wav(path: &Path) -> Result<(WavSpec, Vec<u8>)> {
    let mut file = File::open(path).map_err(|e| ToolError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    let mut header = [0u8; 12];
    file.read_exact(&mut header).map_err(|e| ToolError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
        return Err(ToolError::ExecutionFailed {
            tool_id: "wav_reader".into(),
            reason: format!("Tệp không phải định dạng WAV hợp lệ: {}", path.display()),
        });
    }

    let mut spec: Option<WavSpec> = None;
    let mut data_bytes: Option<Vec<u8>> = None;

    loop {
        let mut chunk_hdr = [0u8; 8];
        match file.read_exact(&mut chunk_hdr) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => {
                return Err(ToolError::Io {
                    path: path.to_path_buf(),
                    source: e,
                })
            }
        }

        let chunk_id = &chunk_hdr[0..4];
        let chunk_size = u32::from_le_bytes(chunk_hdr[4..8].try_into().unwrap()) as usize;

        if chunk_id == b"fmt " {
            let mut fmt_buf = vec![0u8; chunk_size];
            file.read_exact(&mut fmt_buf).map_err(|e| ToolError::Io {
                path: path.to_path_buf(),
                source: e,
            })?;

            if fmt_buf.len() < 16 {
                return Err(ToolError::ExecutionFailed {
                    tool_id: "wav_reader".into(),
                    reason: "Header fmt của tệp WAV quá ngắn".into(),
                });
            }

            let format_tag = u16::from_le_bytes(fmt_buf[0..2].try_into().unwrap());
            // 1 = PCM, 3 = IEEE float
            if format_tag != 1 && format_tag != 3 {
                return Err(ToolError::ExecutionFailed {
                    tool_id: "wav_reader".into(),
                    reason: format!("Chỉ hỗ trợ WAV PCM (nhận được format tag {format_tag})"),
                });
            }

            let channels = u16::from_le_bytes(fmt_buf[2..4].try_into().unwrap());
            let sample_rate = u32::from_le_bytes(fmt_buf[4..8].try_into().unwrap());
            let byte_rate = u32::from_le_bytes(fmt_buf[8..12].try_into().unwrap());
            let block_align = u16::from_le_bytes(fmt_buf[12..14].try_into().unwrap());
            let bits_per_sample = u16::from_le_bytes(fmt_buf[14..16].try_into().unwrap());

            spec = Some(WavSpec {
                channels,
                sample_rate,
                bits_per_sample,
                byte_rate,
                block_align,
            });
        } else if chunk_id == b"data" {
            let mut buf = vec![0u8; chunk_size];
            file.read_exact(&mut buf).map_err(|e| ToolError::Io {
                path: path.to_path_buf(),
                source: e,
            })?;
            data_bytes = Some(buf);
            break; // Finished reading data chunk
        } else {
            // Skip other metadata chunks (LIST, JUNK, etc.)
            let mut skip_buf = vec![0u8; chunk_size];
            file.read_exact(&mut skip_buf).map_err(|e| ToolError::Io {
                path: path.to_path_buf(),
                source: e,
            })?;
        }

        // Align to 2-byte boundary
        if (chunk_size & 1) == 1 {
            let mut pad = [0u8; 1];
            let _ = file.read_exact(&mut pad);
        }
    }

    let parsed_spec = spec.ok_or_else(|| ToolError::ExecutionFailed {
        tool_id: "wav_reader".into(),
        reason: "Không tìm thấy chunk 'fmt ' trong tệp WAV".into(),
    })?;

    let parsed_data = data_bytes.ok_or_else(|| ToolError::ExecutionFailed {
        tool_id: "wav_reader".into(),
        reason: "Không tìm thấy chunk 'data' trong tệp WAV".into(),
    })?;

    Ok((parsed_spec, parsed_data))
}

/// Write PCM audio data with standard 44-byte RIFF WAVE header.
pub fn write_pcm_wav(path: &Path, spec: &WavSpec, data: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ToolError::Io {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }

    let mut file = File::create(path).map_err(|e| ToolError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    let total_file_size = 36 + data.len() as u32;

    // RIFF chunk
    file.write_all(b"RIFF").map_err(|e| ToolError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    file.write_all(&total_file_size.to_le_bytes())
        .map_err(|e| ToolError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
    file.write_all(b"WAVE").map_err(|e| ToolError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    // fmt subchunk
    file.write_all(b"fmt ").map_err(|e| ToolError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    file.write_all(&16u32.to_le_bytes())
        .map_err(|e| ToolError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
    file.write_all(&1u16.to_le_bytes())
        .map_err(|e| ToolError::Io {
            path: path.to_path_buf(),
            source: e,
        })?; // format 1 = PCM
    file.write_all(&spec.channels.to_le_bytes())
        .map_err(|e| ToolError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
    file.write_all(&spec.sample_rate.to_le_bytes())
        .map_err(|e| ToolError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
    file.write_all(&spec.byte_rate.to_le_bytes())
        .map_err(|e| ToolError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
    file.write_all(&spec.block_align.to_le_bytes())
        .map_err(|e| ToolError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
    file.write_all(&spec.bits_per_sample.to_le_bytes())
        .map_err(|e| ToolError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

    // data subchunk
    file.write_all(b"data").map_err(|e| ToolError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    file.write_all(&(data.len() as u32).to_le_bytes())
        .map_err(|e| ToolError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
    file.write_all(data).map_err(|e| ToolError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    file.flush().map_err(|e| ToolError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    Ok(())
}

/// Concatenate multiple WAV PCM files together, inserting zero-filled silence gaps between them.
///
/// Returns the total combined audio duration in seconds.
pub fn concat_wav_files_with_silence(items: &[(&Path, f32)], output_path: &Path) -> Result<f64> {
    if items.is_empty() {
        return Err(ToolError::ExecutionFailed {
            tool_id: "wav_concat".into(),
            reason: "Không có tệp âm thanh nào để ghép nối".into(),
        });
    }

    // Read first file to establish common WavSpec
    let (spec, first_data) = read_pcm_wav(items[0].0)?;
    let mut combined_data = Vec::with_capacity(first_data.len() * items.len());
    combined_data.extend_from_slice(&first_data);

    // If there is silence after the first item, append it
    let first_silence = spec.silence_bytes(items[0].1);
    if first_silence > 0 {
        combined_data.resize(combined_data.len() + first_silence, 0);
    }

    for (idx, (clip_path, pause_s)) in items.iter().enumerate().skip(1) {
        let (clip_spec, clip_data) = read_pcm_wav(clip_path)?;

        if clip_spec.sample_rate != spec.sample_rate
            || clip_spec.channels != spec.channels
            || clip_spec.bits_per_sample != spec.bits_per_sample
        {
            return Err(ToolError::ExecutionFailed {
                tool_id: "wav_concat".into(),
                reason: format!(
                    "Thông số âm thanh không đồng nhất giữa đoạn 0 ({}Hz, {}ch, {}bit) và đoạn {} ({}Hz, {}ch, {}bit)",
                    spec.sample_rate, spec.channels, spec.bits_per_sample,
                    idx, clip_spec.sample_rate, clip_spec.channels, clip_spec.bits_per_sample
                ),
            });
        }

        combined_data.extend_from_slice(&clip_data);

        let silence = spec.silence_bytes(*pause_s);
        if silence > 0 {
            combined_data.resize(combined_data.len() + silence, 0);
        }
    }

    write_pcm_wav(output_path, &spec, &combined_data)?;

    let total_duration = spec.duration_seconds(combined_data.len());
    Ok((total_duration * 1000.0).round() / 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_wav_write_read_roundtrip() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("test_roundtrip.wav");

        let spec = WavSpec {
            channels: 1,
            sample_rate: 22050,
            bits_per_sample: 16,
            byte_rate: 44100,
            block_align: 2,
        };

        // 1 second of audio: 44100 bytes
        let mut sample_data = Vec::with_capacity(44100);
        for i in 0..22050 {
            let val = ((i as f32 * 0.1).sin() * 10000.0) as i16;
            sample_data.extend_from_slice(&val.to_le_bytes());
        }

        write_pcm_wav(&path, &spec, &sample_data).unwrap();

        let (read_spec, read_data) = read_pcm_wav(&path).unwrap();
        assert_eq!(read_spec, spec);
        assert_eq!(read_data.len(), sample_data.len());
        assert_eq!(read_data, sample_data);
    }

    #[test]
    fn test_concat_wav_with_silence_duration_accuracy() {
        let tmp = tempdir().unwrap();
        let path1 = tmp.path().join("clip1.wav");
        let path2 = tmp.path().join("clip2.wav");
        let out_path = tmp.path().join("combined.wav");

        let spec = WavSpec {
            channels: 1,
            sample_rate: 24000,
            bits_per_sample: 16,
            byte_rate: 48000,
            block_align: 2,
        };

        // Clip 1: 1.0 second = 48000 bytes
        let data1 = vec![0x11u8; 48000];
        write_pcm_wav(&path1, &spec, &data1).unwrap();

        // Clip 2: 0.5 second = 24000 bytes
        let data2 = vec![0x22u8; 24000];
        write_pcm_wav(&path2, &spec, &data2).unwrap();

        // Pause after clip 1: 0.45s (period pause)
        let items = [(path1.as_path(), 0.45f32), (path2.as_path(), 0.0f32)];

        let total_duration = concat_wav_files_with_silence(&items, &out_path).unwrap();

        // Expected: 1.0 + 0.45 + 0.5 = 1.95s
        let expected_duration = 1.95;
        let diff_ms = ((total_duration - expected_duration).abs() * 1000.0).round();
        assert!(
            diff_ms <= 10.0,
            "Duration diff {diff_ms}ms exceeds ±10ms tolerance (got {total_duration}, expected {expected_duration})"
        );

        // Verify read back
        let (read_spec, read_data) = read_pcm_wav(&out_path).unwrap();
        assert_eq!(read_spec, spec);

        // Expected data length: 48000 + (24000 * 0.45 * 2 = 21600) + 24000 = 93600
        let expected_len = 48000 + 21600 + 24000;
        assert_eq!(read_data.len(), expected_len);

        // Check silence part is all zeroes
        let silence_slice = &read_data[48000..48000 + 21600];
        assert!(silence_slice.iter().all(|&b| b == 0));
    }
}
