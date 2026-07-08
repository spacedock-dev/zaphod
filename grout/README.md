# grout

Grout binds agent state to the rail: it turns agentsview session data and
subspace decision logs into typed rail rows and pipes them into the zellij
session (plan decisions 1–4, `docs/plan-agent-rail.md`). The plugin never
learns about agentsview; grout never learns about zellij beyond invoking
`zellij pipe`. This is the sprint-0 skeleton: one one-shot run — fetch one
session, read one gate log, emit two rows, exit.

## Usage

Run from the repo root, inside the target zellij session:

    go run ./grout [session-id [gate-log]]

Both positionals optional; defaults live in `main.go` (`defaultConfig`).
One JSON object per line, one line per `zellij pipe` invocation, pipe name
`agent-event`:

    {"kind":"session","id":"…","cwd":"…","agent":"…","state":"…","summary":"…","ts":"2026-07-07T05:00:00Z"}
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

Exit 0 when every row's pipe exited 0 within budget; 1 otherwise. Each pipe
runs under a 5s kill timer: on timeout the child is killed, the reason goes
to stderr (`pipe timeout after 5s: kind=session`), and the next row is still
attempted — fire-and-forget, no retry. Broadcast is by pipe name only, never
`--plugin`, which would launch a non-running plugin in the background.

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
