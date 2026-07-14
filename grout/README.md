# grout

Grout is Zaphod's internal adapter package. It turns AgentsView session data
into typed rail rows and invokes `zellij pipe`; the plugin never learns about
AgentsView. `build.sh` compiles the package into the checkout-local
`target/zaphod` artifact.

The binary has two private executable surfaces. `scripts/zellij-new-tab.sh`
starts `zaphod subscribe` after it verifies a fresh rail. The trusted
checkout-local Codex `SessionStart` hook runs `zaphod
register-agent-session`. Do not invoke either command by hand during normal
use. The registrar atomically records the canonical AgentsView ID with the
inherited Zellij session and pane ID. The subscriber admits only registered
IDs whose panes belong to its stable tab, fetches each ID exactly, and sends
one authoritative snapshot through the named `agent-event` pipe. Gate
delivery remains separate.

The existing `agent-event` protocol carries session and gate rows, but this
first sidecar emits only the session form:

    {"kind":"session","id":"…","pane_id":4,"cwd":"…","agent":"…","state":"…","summary":"…","ts":"2026-07-07T05:00:00Z"}
    {"kind":"gate","log_path":"/abs/…/x.decisions.jsonl","workflow":"…","entity":"…","entity_title":"…","stage":"…","round":1,"recommendation":"…","ts":"2026-07-07T05:00:00Z"}

## Session state

The row's `state` maps agentsview's `termination_status` plus transcript
recency (`ended_at ?? started_at ?? created_at`) onto the plugin's marker
vocabulary; first match wins: `awaiting_user` → `blocked`; last activity
< 60s → `working`; `clean` → `done`; < 10 min → `idle`; otherwise the
status passes through verbatim and the rail shows the unknown marker.
Thresholds mirror agentsview v0.36.1's own liveness derivation; vocabulary
surveyed 2026-07-08 over a 20k-session corpus (awaiting_user, clean,
tool_call_pending, absent; `truncated` exists in code, unobserved).

## Failure posture

The sidecar owns one SSE connection, exact record fetches, target probes, and
short-lived pipe children. Initial connection and each `data_changed` event
produce a complete snapshot from the current registry and fresh native pane
inventory. It never calls the global session list. Target loss, source EOF,
a source error, corrupt or conflicting registry state, exact-record mismatch,
or a pipe error ends the sidecar; it does not reconnect, retry, guess,
retarget, or clean up external resources. Source and pipe operations have
bounded time and size limits. Delivery remains a named-pipe broadcast with an
exact `recipient-tab-id`, never `--plugin`, which would launch an absent
plugin.

The registry root is private (`0700`), its versioned session files and locks
are `0600`, and updates use a bounded advisory lock plus same-directory
write/sync/rename. One pane has one current top-level session. A later startup
or resume in that pane replaces it. Duplicate live claims, malformed input,
over-limit records, and stale pane claims fail closed; stale claims are pruned
against native pane inventory. A full native-session replacement must discard
its ephemeral registry generation before reusing the same Zellij session
name.

## Fixture provenance

- `testdata/session-get.json` — recorded 2026-07-07 off
  `/opt/homebrew/bin/agentsview` v0.36.1 (commit 4c4bb56), isolated from any
  real data: `AGENTSVIEW_DATA_DIR=<tmp> CLAUDE_PROJECTS_DIR=<tmp>`, a
  synthetic 2-line session JSONL under `<tmp>/<munged-cwd>/<uuid>.jsonl`,
  then `agentsview session sync <path>` and
  `agentsview session get <id> --format json`. Refresh the same way, or
  against a real session id from an unsandboxed shell:
  `agentsview session get <id> --format json > grout/testdata/session-get.json`
  (note the binary version here when you do).
- `testdata/playground-gate.{md,decisions.jsonl}` — vendored from the
  `spacedock-research/spacedock-subspace` working copy with that repo at
  HEAD 9be5fbc. The pair is the playground demo's artifact (gitignored
  there as `/playground*`, so not itself in the 9be5fbc tree); its shapes
  match the brief/decision-log code at 9be5fbc (`internal/brief/brief.go`).
