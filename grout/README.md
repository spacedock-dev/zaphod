# grout

Grout is Zaphod's native adapter. `build.sh` compiles it as
`target/zaphod`. The rail remains independent of AgentsView.

The binary exposes two commands:

- `zaphod watch-tab` starts one watcher for the terminal that invokes it.
  The default command returns after a background child becomes ready;
  `--foreground` supports tests and diagnostics.
- `zaphod register-agent-session` accepts the trusted Codex `SessionStart`
  hook and sends it to that terminal's live watcher.

## Authority

The watcher inherits `ZELLIJ_SESSION_NAME` and `ZELLIJ_PANE_ID`. Direct entry
also injects the exact rail URL, recipient token, Zellij profile, AgentsView
URL, and private socket root. The watcher resolves one terminal, its stable
tab, and the original rail. It stops if any member of that tuple disappears
or changes.

The hook sends one bounded, versioned envelope through a `0600` Unix socket
whose name hashes the Zellij session and terminal pane. The socket root is
`0700`. The watcher admits only Codex `SessionStart` records from `startup` or
`resume`, keeps one registration in memory, and fetches only
`/api/v1/sessions/{codex:<UUID>}`. Restart starts empty; no registry,
rehydration, CWD match, title match, or global session list can authorize a
row.

## Delivery and failure

Every session snapshot carries the stable recipient token, tab ID, watcher
generation, and a short lease. The plugin displays and focuses a session row
only while that lease, recipient, pane, and generation remain valid. Heartbeat
renewal uses the cached exact record on a 1.8-second cadence. It is a
recipient- and generation-checked fire-and-forget pipe, so an idle watcher
does not wait for plugin output. AgentsView data changes and new SessionStart
records trigger acknowledged snapshots and exact fetches.

Source EOF, source errors, socket loss, malformed input, over-limit input,
exact-ID mismatch, or rejected snapshot delivery ends the watcher. The plugin
uses its exact manifest and lease to clear rows and focus after terminal, tab,
rail, or watcher loss. Startup, SessionStart/data-change delivery, and cleanup
run bounded native probes; idle heartbeats do not poll pane metadata. A daemon
whose pane disappears silently may remain until the next lifecycle check or
explicit cleanup. HTTP operations, pipe children, cleanup, and records have
explicit time or size bounds.

The existing `agent-event` protocol carries session and gate rows:

```json
{"kind":"session","id":"…","pane_id":4,"cwd":"…","agent":"…","state":"…","summary":"…","ts":"2026-07-07T05:00:00Z"}
{"kind":"gate","log_path":"/abs/…/x.decisions.jsonl","workflow":"…","entity":"…","entity_title":"…","stage":"…","round":1,"recommendation":"…","ts":"2026-07-07T05:00:00Z"}
```

Gate delivery remains separate.

## Session state

The row's `state` maps AgentsView's `termination_status` and transcript
recency (`ended_at ?? started_at ?? created_at`) onto the rail vocabulary.
`awaiting_user` becomes `blocked`; activity younger than 60 seconds becomes
`working`; `clean` becomes `done`; activity younger than ten minutes becomes
`idle`; other values pass through.

## Fixture provenance

- `testdata/session-get.json` records AgentsView v0.36.1 session output from
  2026-07-07.
- `testdata/playground-gate.{md,decisions.jsonl}` records the Subspace gate
  artifact shape used by the gate-row tests.
