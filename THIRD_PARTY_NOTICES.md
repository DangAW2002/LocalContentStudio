# Third-party notices

## open-design

Parts of the chat UI and the web event stream are ported from
[open-design](https://github.com/nexu-io/open-design) (commit `78d5637`),
Copyright the open-design contributors, licensed under the
[Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0).

Ported files (each carries a header naming its upstream source and what changed):

| File | Upstream |
|---|---|
| `packages/desktop-ui/src/runtime/chat/stick-to-bottom.ts` | `apps/web/src/runtime/chat/stick-to-bottom.ts` (logic unchanged, comments condensed) |
| `packages/desktop-ui/src/runtime/chat/jump-to-latest.ts` | `apps/web/src/runtime/chat/jump-to-latest.ts` (logic unchanged) |
| `packages/desktop-ui/src/runtime/chat/useStickToBottom.ts` | DOM wiring extracted from `apps/web/src/components/ChatPane.tsx` |
| `packages/desktop-ui/src/runtime/markdown.tsx`, `markdown.css` | `apps/web/src/runtime/markdown.tsx`, chat/viewer CSS (Vietnamese labels, `::code-comment` dropped) |
| `packages/desktop-ui/src/runtime/shiki.ts` | `apps/web/src/runtime/shiki.ts` |
| `packages/desktop-ui/src/runtime/chat/send-queue.ts` | Queue semantics of `apps/web/src/components/ProjectView.tsx` |
| `crates/studio-web/src/events.rs` | Replay design of `apps/daemon/src/runtimes/runs.ts` (reimplemented in Rust) |
| `packages/desktop-ui/tests/runtime/**` (except `send-queue.test.ts`) | `apps/web/tests/runtime/**` |

open-design's stick-to-bottom state machine is itself derived from
[use-stick-to-bottom](https://github.com/stackblitz-labs/use-stick-to-bottom) (MIT) and
[assistant-ui](https://github.com/assistant-ui/assistant-ui) (MIT).
