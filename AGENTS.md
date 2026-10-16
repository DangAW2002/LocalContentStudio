# AGENTS.md — Local Content Studio

Personal, AI-first desktop studio (Tauri 2 + Rust core + React). Source of truth for design:
`D:/Users/Dang/LocalContentStudio/.aw/` (read-only for you): `plans/2026-10-15-0009-personal-ai-first-reset.md`,
`decisions/ADR-007..ADR-010`, and the task file you are given. If code and docs disagree, follow the docs and say so in your report.

## Crate map (new architecture)

| Crate | Owns |
| --- | --- |
| `studio-workspace` | Org/workspace folders, scope chain app→org→workspace, path scoping, SQLite (`.studio/studio.db`, `app.db`), migrations, file index, checkpoints (CAS), disk GC |
| `studio-jobs` | Job scheduler, resource pools, admission, Windows Job Objects, process supervision, `Worker` trait |
| `studio-engine` | Agent engine adapters (`claude`, `agy`): spawn CLI, parse stream-json, normalized `Event`s |
| `studio-catalog` | Skill catalog (`catalog/catalog.yaml`), inventory, curation checks, kit resolution, per-turn skill plugin dir |
| `studio-tools` | Capability + tool registry, manifests, provider routing, secrets (Credential Manager), tool proxy |
| `studio-brand` | Brand/Channel/Playbook schemas (brand@1, channel@1), loading via scope chain, checkers (palette, contrast, logo, safe zone, size/duration, char limits, captions, voice words) |
| `studio-artifacts` | artifact.yaml model, kind registry, render dispatch (design HTML→PNG/PDF, motion HyperFrames, timeline FFmpeg), render cache by input hash, proxies |
| `studio-mcp` | Studio MCP server (Streamable HTTP loopback + token; stdio shim) |
| `studio-tauri` | Desktop shell + IPC |
| `studio-cli` | `studio` CLI |
| `packages/desktop-ui` | React UI |

Legacy crates of the previous multi-user design were removed on 2026-10-16 (see git history).

## Rules

- Touch only the crate(s)/paths named in your task. Do not edit the root `Cargo.toml` workspace members; declare dependencies with
  explicit versions in your own crate's `Cargo.toml` (you may use `{ workspace = true }` for deps already in `[workspace.dependencies]`).
- Library code: no `unwrap()`/`expect()` outside tests; errors via `thiserror`; public API documented with `///`.
- Windows is the primary target (paths, process trees, Credential Manager). Code must still compile on other targets
  (`#[cfg(windows)]` with a simple fallback).
- Never fake capabilities: probes must run the real binary; tests for external CLIs use recorded fixtures.
- Never write secrets to files, logs or test fixtures.
- UI copy is Vietnamese; no raw enums shown to users.

## Gate (must pass before you report done)

```
cargo fmt --all -- --check
cargo clippy -p <your-crate> --all-targets -- -D warnings
cargo test -p <your-crate>
```
For UI: `pnpm --filter @studio/desktop-ui typecheck` and `pnpm --filter @studio/desktop-ui build`.

Do not commit; the orchestrator reviews and commits. Report: what you built, public API, tests, gate output, known gaps.
