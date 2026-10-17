#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""VieNeu-TTS v3 Turbo offline speech synthesis helper script.

Reads synthesis text from a temporary UTF-8 file (avoiding shell escaping issues)
and outputs audio WAV to the designated target path.
"""

import argparse
import json
import os
import sys


def main():
    parser = argparse.ArgumentParser(description="VieNeu-TTS synthesis helper")
    parser.add_argument(
        "--text-file", required=True, help="Path to input text file (UTF-8)"
    )
    parser.add_argument(
        "--output-file", required=True, help="Path to output WAV file"
    )
    parser.add_argument(
        "--voice", default="Bình", help="Preset voice name or cloning target"
    )
    parser.add_argument(
        "--speed", type=float, default=1.0, help="Speech speed multiplier"
    )
    parser.add_argument(
        "--ref-audio",
        default=None,
        help="Optional reference audio path for voice cloning",
    )
    parser.add_argument(
        "--metadata-file",
        default=None,
        help="Optional path to write metadata JSON",
    )

    args = parser.parse_args()

    if not os.path.isfile(args.text_file):
        sys.stderr.write(f"Tệp văn bản không tồn tại: {args.text_file}\n")
        sys.exit(1)

    try:
        with open(args.text_file, "r", encoding="utf-8") as f:
            text = f.read().strip()
    except Exception as e:
        sys.stderr.write(f"Không thể đọc tệp văn bản UTF-8: {e}\n")
        sys.exit(1)

    if not text:
        sys.stderr.write("Nội dung văn bản để đọc rỗng.\n")
        sys.exit(1)

    try:
        import vieneu
    except ImportError:
        sys.stderr.write(
            "Chưa cài đặt thư viện 'vieneu'. Hãy chạy lệnh: pip install vieneu\n"
        )
        sys.exit(2)

    try:
        if hasattr(vieneu, "Vieneu"):
            tts = vieneu.Vieneu()
        elif hasattr(vieneu, "VieNeuTTS"):
            tts = vieneu.VieNeuTTS()
        else:
            raise RuntimeError(
                "Không tìm thấy class Vieneu hoặc VieNeuTTS trong gói vieneu"
            )
    except Exception as e:
        sys.stderr.write(f"Lỗi khi khởi tạo VieNeu-TTS: {e}\n")
        sys.exit(2)

    infer_kwargs = {"text": text}
    actual_voice = args.voice

    if args.ref_audio:
        if not os.path.isfile(args.ref_audio):
            sys.stderr.write(
                f"Tệp âm thanh mẫu không tồn tại: {args.ref_audio}\n"
            )
            sys.exit(1)
        infer_kwargs["ref_audio"] = args.ref_audio
        actual_voice = f"clone:{os.path.basename(args.ref_audio)}"
    else:
        infer_kwargs["voice"] = args.voice

    if args.speed != 1.0:
        infer_kwargs["speed"] = args.speed

    try:
        audio = tts.infer(**infer_kwargs)
    except Exception as e:
        sys.stderr.write(f"Lỗi khi sinh âm thanh qua VieNeu-TTS: {e}\n")
        sys.exit(3)

    out_path = os.path.abspath(args.output_file)
    out_dir = os.path.dirname(out_path)
    if out_dir and not os.path.exists(out_dir):
        try:
            os.makedirs(out_dir, exist_ok=True)
        except Exception as e:
            sys.stderr.write(
                f"Không thể tạo thư mục đích '{out_dir}': {e}\n"
            )
            sys.exit(4)

    try:
        tts.save(audio, out_path)
    except Exception as e:
        sys.stderr.write(f"Không thể ghi tệp âm thanh '{out_path}': {e}\n")
        sys.exit(4)

    result_meta = {
        "audio_path": out_path,
        "provider": "tts.vieneu",
        "voice": actual_voice,
        "available_voices": ["Bình", "Mai", "Nam", "Nữ"],
        "note": "Tổng hợp giọng nói thành công qua VieNeu-TTS v3 Turbo.",
    }

    if args.metadata_file:
        try:
            with open(args.metadata_file, "w", encoding="utf-8") as mf:
                json.dump(result_meta, mf, ensure_ascii=False)
        except Exception:
            pass

    print(json.dumps(result_meta, ensure_ascii=False))


if __name__ == "__main__":
    main()
