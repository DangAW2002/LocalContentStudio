# Local Content Studio — Claude Instructions

## Git Commit Rules
- STRICT: NEVER include `Co-Authored-By` lines or any bot/AI attribution trailers (e.g. `Co-Authored-By: Claude ...`) in git commit messages.
- The repository has a `commit-msg` hook that strictly blocks any commit containing `Co-Authored-By`.
- Always format commit messages concisely: `<type>(<scope>): <summary>` followed by optional body if needed.

## CodeGraph
<!-- CODEGRAPH_START -->
In repositories indexed by CodeGraph (a `.codegraph/` directory exists at the repo root), reach for it BEFORE grep/find or reading files when you need to understand or locate code:
- MCP tools: `codegraph_explore`, `codegraph_node`
- Shell: `codegraph explore "<symbol names or question>"` and `codegraph node <symbol-or-file>`
If there is no `.codegraph/` directory, skip CodeGraph entirely.
<!-- CODEGRAPH_END -->
