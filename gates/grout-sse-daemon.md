---
subspace: v0
gate:
  workflow: agent-rail-dev
  entity: grout-sse-daemon
  entity-title: Grout SSE daemon — sessions for real
  stage: validation
  round: 1
recommendation:
  verdict: PENDING-DEMO
  rationale: Offline AC-1..5 independently re-run PASS; refutation audit found no REFUTED against them (one non-blocking parser-level finding on truncated SSE frames, mitigated end-to-end by reconnect-always-refreshes); AC-6/AC-7 await your live demo and AC-8 awaits a real dogfood window (script and framing in the artifact).
artifact:
  kind: draft
  path: ./grout-sse-daemon-validation.md
criteria:
  bar: |
    The demo is the gate. Offline ACs must be independently re-run by a
    fresh agent (not re-read from the implementer's report), a refutation
    audit must run on a throwaway checkout — never the implementation
    worktree — naming concrete attack scenarios attempted, and the
    interactive ACs are settled only by CL: a live demo in a fresh zellij
    session for AC-6/AC-7, and a real dogfood window for AC-8.
  acceptance:
    - id: AC-1
      text: "Value, wire-level: against a scripted SSE stream and a fake agentsview whose list responses change across refreshes, grout watch emits rows for appearing sessions, an updated row within 5s of the revealing data_changed, and stops refreshing a session once it leaves the fixture list while others keep re-emitting with strictly increasing ts."
      evidence: "cd grout && go test -run TestWatchSessionsFlow -v ./...: PASS"
    - id: AC-2
      text: "Attach-or-start and reconnect: with no SSE server listening, grout execs `agentsview serve --background` and connects once it appears; a dropped stream reconnects with backoff and runs a full refresh; sustained silence past a threshold forces reconnect."
      evidence: "cd grout && go test -run 'TestWatchAutoStart|TestWatchReconnect' -v ./...: PASS (plus bonus TestWatchReconnectOnSilence PASS)"
    - id: AC-3
      text: "A wedged rail cannot wedge or flood the watcher: refreshes stay single-flight, a burst of data_changed during a stuck refresh coalesces to exactly one follow-up, and SIGTERM ends the process within 10s leaving no children."
      evidence: "cd grout && go test -run TestWatchWedgedPipe -v ./...: PASS"
    - id: AC-4
      text: "Cross-tab bind and click decisions: a unique own-tab cwd match binds there even when a foreign tab also matches; own-tab zero plus exactly one foreign match binds cross-tab with FocusPane; duplicate matches anywhere render unbound with a dead click."
      evidence: "cargo test bind_cross_tab: 4/4 PASS; full cargo test: 136/136 PASS; cargo check --tests: clean"
    - id: AC-5
      text: "One-shot mode regression-free: the existing grout suite passes unchanged under GOPROXY=off."
      evidence: "cd grout && GOPROXY=off go test -count=1 ./... && go vet ./...: PASS, clean (fresh run, no cache); git diff confirms gate.go/rows.go/agentsview.go untouched"
    - id: AC-6
      text: "Value: in CL's fresh zellij session, one grout watch start fills AGENTS within 30s with the same session count an independent `agentsview session list --server` baseline reports at that moment; a brand-new session in another tab appears within 30s of its first message with no grout interaction."
      evidence: "pending — demo script in the artifact; settled only by CL's live drive. A live spot-check in this review (real agentsview daemon, not fakes) showed sub-second appearance and exact count parity, de-risking but not substituting for CL's fresh-session run."
    - id: AC-7
      text: "Cross-tab click lands the operator on the agent: clicking a session row whose bound pane lives in another tab switches the view to that tab with the pane focused."
      evidence: "pending — demo script in the artifact; the one mechanism that could not be reproduced offline or in this review's headless sandbox (focus_terminal_pane's tab-switch is inherently visual, no live zellij session available here)"
    - id: AC-8
      text: "Dogfood exit: after a real working day with two or more concurrent workflows, CL confirms at the gate that finding the next blocked agent went through the rail, not tab-scanning."
      evidence: "pending — framing question in the artifact; settled only by CL's lived dogfood experience, not a single-sitting demo"
---

Offline AC-1..5 were independently re-run in this review (not re-read from
the implementer's stage report) against the implementation worktree at
`d08d8b55205bc637a61e5659089268752293f321` — all PASS, both the Go and Rust
suites. A refutation audit ran on a fresh throwaway checkout at the same
commit (never the implementation worktree): six adversarial probes
attempted (truncated final SSE frame, malformed session-list JSON, render
line-index bounds, `"watch"` argv shadowing one-shot's SessionID, an
own-tab-duplicate coverage gap, and a `serve stop` grep). One real
parser-level false negative surfaced (a truncated final SSE frame is
silently dropped) but does not survive as an end-to-end defect — reconnect
always forces a full refresh, independently confirmed by an existing test.
No REFUTED against AC-1..5.

AC-6 and AC-7 are the live interactive exit (this task changes `src/main.rs`,
so a plugin rebuild is required before the demo — steps in the artifact,
build verified clean in this review). AC-8 settles from CL's lived
dogfood experience, not a script. Full script and evidence in the artifact.
