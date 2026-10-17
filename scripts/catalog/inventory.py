"""Inventory third-party skill/MCP sources for the Studio catalog.

Reads files only; never executes anything inside the sources.
Usage: python -I scripts/catalog/inventory.py <sources_dir> <out_json>
"""

import json
import re
import subprocess
import sys
from pathlib import Path

SCRIPT_EXT = {".py", ".js", ".mjs", ".cjs", ".ts", ".sh", ".ps1", ".bat", ".jsx", ".tsx"}
LICENSE_NAMES = ("LICENSE", "LICENSE.md", "LICENSE.txt", "LICENCE", "license", "LICENSE.TXT")


def frontmatter(text: str) -> dict:
    m = re.match(r"^---\s*\n(.*?)\n---\s*\n", text, re.S)
    if not m:
        return {}
    out, key = {}, None
    for line in m.group(1).splitlines():
        kv = re.match(r"^([A-Za-z0-9_-]+):\s*(.*)$", line)
        if kv:
            key = kv.group(1)
            out[key] = kv.group(2).strip().strip('"').strip("'")
        elif key and line.startswith((" ", "\t")):
            out[key] = (out[key] + " " + line.strip()).strip()
    return out


def license_of(dirpath: Path) -> str | None:
    for name in LICENSE_NAMES:
        f = dirpath / name
        if f.is_file():
            head = f.read_text(encoding="utf-8", errors="replace").strip().splitlines()[:3]
            return " ".join(h.strip() for h in head)[:120]
    return None


def tokens(chars: int) -> int:
    return max(1, chars // 4)


def repo_entry(repo: Path) -> dict:
    commit = subprocess.run(
        ["git", "-C", str(repo), "rev-parse", "HEAD"], capture_output=True, text=True
    ).stdout.strip()
    skills = []
    for md in sorted(repo.rglob("SKILL.md")):
        if ".git" in md.parts:
            continue
        text = md.read_text(encoding="utf-8", errors="replace")
        fm = frontmatter(text)
        sdir = md.parent
        files = [p for p in sdir.rglob("*") if p.is_file()]
        scripts = sorted(str(p.relative_to(sdir)) for p in files if p.suffix.lower() in SCRIPT_EXT)
        skills.append(
            {
                "path": str(sdir.relative_to(repo)).replace("\\", "/"),
                "name": fm.get("name") or sdir.name,
                "description": fm.get("description", ""),
                "desc_tokens": tokens(len(fm.get("description", ""))),
                "body_tokens": tokens(len(text)),
                "bundle_tokens": tokens(
                    sum(p.stat().st_size for p in files if p.suffix.lower() in {".md", ".txt"})
                ),
                "scripts": scripts[:20],
                "script_count": len(scripts),
                "license": license_of(sdir) or fm.get("license"),
            }
        )
    markers = {
        "claude_plugin": (repo / ".claude-plugin").exists(),
        "package_json": (repo / "package.json").exists(),
        "pyproject": (repo / "pyproject.toml").exists(),
        "agents_dir": (repo / ".agents").exists() or (repo / "agents").exists(),
    }
    return {
        "repo": repo.name.replace("__", "/", 1),
        "commit": commit,
        "license": license_of(repo),
        "markers": markers,
        "skill_count": len(skills),
        "desc_tokens_total": sum(s["desc_tokens"] for s in skills),
        "skills": skills,
    }


def main() -> None:
    src, out = Path(sys.argv[1]), Path(sys.argv[2])
    repos = [repo_entry(p) for p in sorted(src.iterdir()) if (p / ".git").exists() or p.name == "studio__local"]
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(repos, ensure_ascii=False, indent=2), encoding="utf-8")
    for r in repos:
        print(
            f"{r['repo']:<36} {r['commit'][:7]} skills={r['skill_count']:<3} "
            f"desc_tok={r['desc_tokens_total']:<5} license={(r['license'] or '-')[:40]!r} "
            f"markers={[k for k, v in r['markers'].items() if v]}"
        )


if __name__ == "__main__":
    main()
