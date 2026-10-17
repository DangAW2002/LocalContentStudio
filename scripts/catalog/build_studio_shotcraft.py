"""Build the Studio-owned skill pack derived from Vincentwei1021/video-shotcraft (Apache-2.0).

Copies the renderer-agnostic knowledge (shot recipe cards, demo TSX as reference implementations,
pipeline / aesthetic / sound references) into catalog/studio/skills/, generates a shot index, and
records provenance. Hand-written SKILL.md files in those folders are never overwritten.

Usage: python -I scripts/catalog/build_studio_shotcraft.py <video-shotcraft clone> [<commit>]
"""

from __future__ import annotations

import json
import shutil
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
OUT = REPO / "catalog" / "studio"
UPSTREAM = "Vincentwei1021/video-shotcraft"

CATEGORY_VI = {
    "opening": "Mở đầu & thương hiệu",
    "typography": "Chữ & thẻ tiêu đề",
    "ui-entrance": "Giao diện xuất hiện & trưng bày",
    "camera": "Máy quay & không gian",
    "data": "Số liệu & chỉ số",
    "interaction": "Tương tác & demo tính năng",
    "transition": "Chuyển cảnh",
    "rhythm": "Nhịp & montage",
    "effects": "Ánh sáng & nhấn mạnh",
    "outro": "Kết",
}

# (source relative path, destination relative to catalog/studio/skills)
DOCS = [
    ("references/pipeline.md", "promo-pipeline/references/pipeline.md"),
    ("references/guided-free-creation.md", "promo-pipeline/references/guided-free-creation.md"),
    ("references/final-review.md", "promo-pipeline/references/final-review.md"),
    ("references/sequences/promo-energy-arc.md", "promo-pipeline/references/promo-energy-arc.md"),
    ("references/aesthetic-rules.md", "motion-aesthetics/references/aesthetic-rules.md"),
    ("references/sound-design.md", "motion-aesthetics/references/sound-design.md"),
    ("references/music-beat-sync.md", "motion-aesthetics/references/music-beat-sync.md"),
    ("references/shots/ATTRIBUTION.md", "shot-recipes/cards/ATTRIBUTION.md"),
]


def copy_tree(src: Path, dst: Path, suffixes: tuple[str, ...], skip_parts: tuple[str, ...] = ()) -> int:
    n = 0
    for f in sorted(src.rglob("*")):
        if not f.is_file() or f.suffix not in suffixes:
            continue
        rel = f.relative_to(src)
        if any(p in skip_parts for p in rel.parts):
            continue
        target = dst / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(f, target)
        n += 1
    return n


def main() -> None:
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    src = Path(sys.argv[1]).resolve()
    commit = sys.argv[2] if len(sys.argv) > 2 else "unknown"
    skills = OUT / "skills"

    # Regenerate only the derived folders; keep hand-written SKILL.md files.
    for derived in ["shot-recipes/cards", "shot-recipes/demos", "shot-recipes/lib",
                    "promo-pipeline/references", "motion-aesthetics/references"]:
        shutil.rmtree(skills / derived, ignore_errors=True)

    cards = copy_tree(src / "references/shots", skills / "shot-recipes/cards", (".md",))
    # Reference implementations: TSX/TS only (no textures, no retired _legacy shots).
    demos = copy_tree(src / "demos", skills / "shot-recipes/demos", (".tsx", ".ts", ".md"), ("_legacy", "_textures"))
    lib = copy_tree(src / "assets/lib", skills / "shot-recipes/lib", (".tsx", ".ts", ".md"))
    for rel_src, rel_dst in DOCS:
        target = skills / rel_dst
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(src / rel_src, target)

    library = json.loads((src / "gallery/api/library.json").read_text(encoding="utf-8"))
    rows: dict[str, list[str]] = {}
    for c in library["cards"]:
        cell = lambda v: str(v or "").replace("|", "/").replace("\n", " ").strip()
        rows.setdefault(c["category"], []).append(
            f"| `{c['name']}` | {cell(c.get('summary'))} | {cell(c.get('duration'))} | {cell(c.get('energy'))} |"
        )
    lines = [
        "# Mục lục thẻ cảnh quay",
        "",
        f"Sinh tự động từ `{UPSTREAM}` @ `{commit}` — {len(library['cards'])} thẻ. Đọc thẻ đầy đủ ở",
        "`cards/<nhóm>/<tên>.md`, mã tham chiếu ở `demos/<nhóm>/<tên>/` (Remotion TSX).",
        "Tóm tắt giữ nguyên văn gốc (tiếng Trung).",
        "",
    ]
    for cat, title in CATEGORY_VI.items():
        if cat not in rows:
            continue
        lines += [f"## {title} (`{cat}`) — {len(rows[cat])} thẻ", "",
                  "| Thẻ | Tóm tắt | Thời lượng | Năng lượng |", "| --- | --- | --- | --- |", *rows[cat], ""]
    (skills / "shot-recipes/INDEX.md").write_text("\n".join(lines), encoding="utf-8", newline="\n")

    shutil.copyfile(src / "LICENSE", OUT / "LICENSE.video-shotcraft")
    (OUT / "NOTICE.md").write_text(
        "# NOTICE\n\n"
        f"Các thư mục `skills/shot-recipes/{{cards,demos,lib}}`, `skills/promo-pipeline/references` và\n"
        f"`skills/motion-aesthetics/references` được sao chép nguyên văn từ\n"
        f"[{UPSTREAM}](https://github.com/{UPSTREAM}) @ `{commit}` (tác giả Wei Yihao), giấy phép\n"
        "Apache-2.0 (xem `LICENSE.video-shotcraft`). Các tệp `SKILL.md` là bản viết lại của Studio.\n"
        "Không sao chép: audio (một số SFX chưa xác minh nguồn), texture, gallery media, workbench,\n"
        "phần quảng bá tác giả, module đăng mạng xã hội.\n",
        encoding="utf-8", newline="\n",
    )
    print(f"cards={cards} demos={demos} lib={lib} docs={len(DOCS)} index_rows={len(library['cards'])}")


if __name__ == "__main__":
    main()
