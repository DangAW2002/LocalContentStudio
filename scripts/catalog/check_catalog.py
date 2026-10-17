"""Validate catalog.yaml against the source inventory and report per-kit noise budget.

Usage: python -I scripts/catalog/check_catalog.py <catalog.yaml> <inventory.json>
Exit code 1 when references are broken, skills are uncurated, or a kit exceeds the budget.
"""

import json
import sys

import yaml

MAX_VISIBLE_SKILLS = 15
MAX_DESC_TOKENS = 2000
ROUTER_DESC_TOKENS = 60  # estimate for a generated router description


def main() -> int:
    catalog = yaml.safe_load(open(sys.argv[1], encoding="utf-8"))
    inventory = json.load(open(sys.argv[2], encoding="utf-8"))
    skills = {(r["repo"], s["name"]): s for r in inventory for s in r["skills"]}
    problems: list[str] = []

    # Bundled (kind: local) sources have no git checkout to drift from.
    pinned = {repo: src["commit"] for repo, src in catalog["sources"].items() if src.get("kind") != "local"}
    for r in inventory:
        if r["repo"] in pinned and pinned[r["repo"]] != r["commit"]:
            problems.append(f"commit drift {r['repo']}: catalog {pinned[r['repo']][:7]} != local {r['commit'][:7]}")

    curated: set[tuple[str, str]] = set()
    for repo, names in catalog.get("excluded", {}).items():
        curated |= {(repo, n) for n in names}
    for key in catalog.get("adapt", {}):
        repo, name = key.rsplit("/", 1)
        curated.add((repo, name))

    module_skills: dict[str, list[tuple[str, str]]] = {}
    for mid, mod in catalog["modules"].items():
        refs = [(repo, n) for repo, names in (mod.get("skills") or {}).items() for n in names]
        for ref in refs:
            if ref not in skills:
                problems.append(f"module {mid}: unknown skill {ref[0]}/{ref[1]}")
        curated |= set(refs)
        module_skills[mid] = refs
        for dep in mod.get("requires", []):
            if dep not in catalog["modules"]:
                problems.append(f"module {mid}: unknown required module {dep}")

    for ref in sorted(set(skills) - curated):
        problems.append(f"uncurated skill {ref[0]}/{ref[1]}")

    print(f"{'kit':<20} {'visible':>7} {'desc_tok':>8} {'loaded':>6}  modules")
    for kid, kit in catalog["kits"].items():
        active: dict[str, list[tuple[str, str]]] = {}
        for mid, sel in kit["use"].items():
            refs = module_skills[mid]
            chosen = refs if sel == "all" else [r for r in refs if r[1] in sel]
            if sel != "all":
                for n in set(sel) - {r[1] for r in refs}:
                    problems.append(f"kit {kid}: {mid} has no skill {n}")
            active.setdefault(mid, []).extend(chosen)
            mod = catalog["modules"][mid]
            for name, deps in (mod.get("skill_requires") or {}).items():
                if name not in {r[1] for r in chosen}:
                    continue
                for dep in deps:
                    dmid, _, dname = dep.partition("/")
                    dep_refs = module_skills[dmid]
                    active.setdefault(dmid, []).extend(r for r in dep_refs if not dname or r[1] == dname)
            for req in mod.get("requires", []):
                active.setdefault(req, [])
        visible = desc = loaded = 0
        for mid, refs in active.items():
            refs = list(dict.fromkeys(refs))
            loaded += len(refs)
            router = catalog["modules"][mid].get("router")
            if router and len(refs) > 1:
                visible += 1
                desc += ROUTER_DESC_TOKENS if router == "generated" else skills[tuple(router.rsplit("/", 1))]["desc_tokens"]
            else:
                visible += len(refs)
                desc += sum(skills[r]["desc_tokens"] for r in refs)
        flag = "" if visible <= MAX_VISIBLE_SKILLS and desc <= MAX_DESC_TOKENS else "  OVER BUDGET"
        if flag:
            problems.append(f"kit {kid} over budget: {visible} visible, {desc} tokens")
        print(f"{kid:<20} {visible:>7} {desc:>8} {loaded:>6}  {', '.join(active)}{flag}")

    all_desc = sum(s["desc_tokens"] for s in skills.values())
    print(f"\nall sources: {len(skills)} skills, {all_desc} description tokens if everything were loaded")
    for p in problems:
        print("PROBLEM:", p)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
