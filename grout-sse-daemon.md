---
title: Grout SSE daemon — sessions for real
status: validation
source: plan sprint 1 (docs/plan-agent-rail.md)
score: 0.8
id: ybqh2eyp10qjbv05bd7njtbw
started: 2026-07-07T23:00:50Z
worktree: .worktrees/spacedock-ensign-grout-sse-daemon
---

## Problem

Grout is one-shot: one session fetched by hand, one gate log, run manually
per demo. The rail's daily value needs every active session appearing,
updating, and disappearing at the terminal edge without CL running anything —
and a click that lands him on the blocked agent even when its pane lives in
another tab. Sprint-1 exit: CL stops alt-tabbing to find blocked agents.

## Proposed approach

Long-running `grout watch`: attach to (or start) the agentsview daemon,
consume SSE `data_changed` from `/api/v1/events`, re-list the active session
window via `session list --server`, and emit session rows on `agent-event` —
the skeleton's row protocol, builders, and kill-timer emit path unchanged.
Plugin side: session→pane binding widens from own-tab to all tabs, and a
bound click switches tabs via `focus_terminal_pane`'s documented
tab-switching semantics.

### Mechanism facts pinned at ideation (probe on the record, 2026-07-08)

The checklist's riskiest mechanisms were exercised live during this ideation,
in-sandbox, against the real v0.36.1 binary (commit 4c4bb56) with env
isolation (`AGENTSVIEW_DATA_DIR`/`CLAUDE_PROJECTS_DIR` at a scratch dir; the
sandbox blocks the real data dirs but the binary runs):

1. **Daemon + SSE endpoint.** `agentsview serve --background --port 18080
   --no-browser --no-update-check` starts and reports pid/URL; `serve status`
   reads it back; `serve stop` stops it. The SSE stream is
   `GET /api/v1/events` (UI bundle constructs
   `new EventSource(`${base}/events`)` with base `/api/v1`; a per-session
   `/api/v1/sessions/{id}/watch` stream also exists — unused here).
2. **`data_changed` end to end.** With `curl -N` on `/api/v1/events`, writing
   a synthetic 2-line Claude session JSONL under the watched projects dir
   produced, within ≤12 s (write 23:07:44Z, observed by 23:07:56Z):

       event: data_changed
       data: {"scope":"sessions"}

       event: heartbeat
       data: 2026-07-08T07:07:56+08:00

   Events carry no session content — scope only — and a `heartbeat` event
   type exists (liveness). `serve --help` pins `--events-coalesce-interval`
   default **10s** ("Minimum interval between SSE data_changed broadcasts")
   — the floor on update latency.
3. **Enrichment.** `session list --server http://127.0.0.1:18080 --json
   --include-one-shot --include-automated` returned the synthetic session
   (`{"sessions":[…],"total":1}`, ~60 ms) with
   cwd/agent/termination_status/first_message — the row's source fields;
   `session get <id> --server … --json` likewise. `--active-since <RFC3339>`
   filters server-side through `--server` (past horizon → total 1, future →
   total 0). Default list behavior hides one-shot/automated sessions — the
   include flags are mandatory. (`termination_status: "clean"` observed —
   one more vocabulary value for ka's mapping.)
4. **Cross-tab focus fact re-checked.** zellij-tile 0.44.1 `shim.rs:1384`
   documents `focus_terminal_pane`: "Changes the focus to the terminal pane
   with the specified id, … switching to its tab and layer".
   `go_to_tab(tab_index)` (shim.rs:1354) is the fallback if live behavior
   disappoints; the bind set carries the pane's tab either way.
   `get_pane_cwd(pane_id)` (shim.rs:1935) is global-pane-id-scoped — no tab
   constraint in the contract; foreign-tab reads get live confirmation in
   the demo.

No spike needed beyond the above: transport (sprint-0 spike + shipped
skeleton), row protocol + kill-timer emit (shipped, validated), brief shapes
(subspace recon @9be5fbc). The remaining unproven residue is live-only —
foreign-tab `focus_terminal_pane`/`get_pane_cwd` behavior in a real session —
and is the first item in the test plan.

### Grout: watch mode

`grout watch [-server URL] [-since 30m] [-tick 60s] [-gate-log PATH]` — a
new subcommand; the one-shot invocation and its tests stay untouched.

- **Attach-or-start.** Try the SSE connect; on connection failure run
  `agentsview serve --background --no-browser --no-update-check` once and
  retry with backoff (1s doubling, 30s cap). Grout never runs `serve stop` —
  the daemon is shared, not grout's to kill. This absorbs the
  auto-spawned-daemon-on-:8080 quirk deliberately.
- **SSE client:** net/http + bufio line scanning; a pure `parseSSEEvents`
  over `event:`/`data:` framing (fixture = the probe's recorded bytes above,
  re-recordable via the test-plan recipe). No third-party dep (plan
  decision 2: the client is trivial).
- **Refresh loop.** Refresh on every `data_changed` regardless of scope (a
  refresh costs ~60 ms — cheaper than scope bookkeeping), and on a `-tick`
  wall-clock timer (60s default) — required because a session leaving the
  active window emits no event. Refresh = `session list --server <URL>
  --json --include-one-shot --include-automated --include-children
  --active-since <now − since>` (shelling out to the agentsview binary — the
  enrichment surface the plan pins, since events carry no content), decode
  `{"sessions":[…]}`, build one row per entry via the existing
  `BuildSessionRow`, emit each via the existing `EmitRow` (5s kill timer,
  sequential, never `--plugin` — all shipped semantics).
- **Emit policy: full re-emit per refresh.** Every refresh emits every
  in-window session row with fresh `ts`. The plugin upserts idempotently and
  re-renders only on change; the re-stamped `ts` is the seam the
  row-lifecycle task (hj) consumes for expiry. Self-healing by construction:
  a restarted rail or a dropped pipe converges on the next refresh.
  Worst-case pipe load ≈ N sessions per 10s coalesce window (N≈5–20 active)
  — a few process spawns per second at the ceiling, fine on a dev box.
- **Single-flight refresh.** One refresh in flight; events landing
  mid-refresh set a dirty flag → exactly one follow-up refresh
  (latest-wins). Against a wedged rail each row still burns ≤5s+2s
  sequentially; single-flight bounds the backlog to one queued refresh.
- **Reconnect.** Stream EOF/error → reconnect with the same backoff, full
  refresh on re-establish. No traffic (no event, no heartbeat) for 90s →
  force reconnect.
- **Shutdown:** SIGINT/SIGTERM → finish or kill the in-flight pipe, exit 0.
- **Gate row:** `-gate-log PATH` optional; when given, emit the gate row at
  startup and on each tick exactly as the skeleton does (GateFromLog +
  BuildGateRow). No discovery/folding — sprint 2. When absent, no gate row
  (sidesteps the cwd-relative default; ka fixes the one-shot default).
- **Files:** stay single `package main` (the skeleton's forecast of a
  package split is deferred — YAGNI until a second consumer exists):
  `watch.go` (loop, single-flight, tick), `sse.go` (connect +
  parseSSEEvents), `list.go` (session-list exec + decode). Pure decision
  fns carry the decide_toggle test discipline.

### Plugin: cross-tab binding + click

- **Binding domain widens to all tabs' terminal panes** (same pane filter as
  `rows_for_own_tab`: !is_plugin, is_selectable, !is_suppressed — applied
  per tab across the manifest) via a new pure sibling
  `terminal_panes_all_tabs(manifest)`; `pane_cwds` polling extends over that
  set under the existing wedge budget (one wedge-classified call aborts the
  pass) and per-pane backoff — no new polling mechanism, a bigger set
  (own-tab ~5 → all-tab ~20 on CL's layout).
- **Bind rule (extends `bind_session`):** exactly one cwd match in the
  rail's own tab → bind there (preserves today's behavior — the operator is
  already in front of it); else exactly one match across all tabs → bind
  cross-tab; else unbound (two same-cwd panes in different tabs render
  unbound — honest, never guessed; flap smoothing is hj's).
- **Click (extends `decide_rail_click`):** a bound row click stays
  `ClickAction::FocusPane(id)` → `focus_terminal_pane(id, false, false)`,
  which per the 0.44.1 contract switches to the pane's tab. If the live
  check refutes the view-switch, the fallback is `go_to_tab(tab)` first —
  the bind set carries the tab index. Nav-mode exit on click (F8 fix)
  already shipped.
- Rendering unchanged: bound/unbound marker as shipped; state markers are
  ka's.

### Sibling seams (explicit)

- **ka grout-session-states** owns the `termination_status`→marker
  vocabulary in `BuildSessionRow` — this task emits whatever mapping rows.go
  has when it lands; the demo's "blocked shows blocked" observation depends
  on ka. Dispatch order (0.9 > 0.8) puts ka first.
- **hj rail-row-lifecycle** owns visible row removal (ts-staleness expiry)
  and unbind debounce. This task's obligations to that seam: fresh `ts` on
  every re-emit, and refreshes stopping for out-of-window sessions — after
  which expiry is a pure function of `ts`. Until hj lands, an ended session
  keeps its last row (stale-not-removed) — acceptable interim dogfood.

### Doc diff (proposed)

README.md, the Agent & gate rows bullet — replace the first sentence:

    - **Agent & gate rows**: a companion `grout watch` daemon streams
      `agent-event` rows into the rail — every agent session active in the
      last 30 minutes appears with its state, updates as agentsview sees new
      activity (≈10 s coalescing), and stops refreshing once it leaves the
      window (visible row expiry lands with the row-lifecycle task). Click a
      session row to focus its cwd-bound pane — switching tabs when the pane
      lives elsewhere (unbound is shown, never guessed); click a gate row to
      float `subspace-tui` on the gate's artifact with `--log` pointed at
      its decision log. Requires the `RunCommands` permission (prompted once)

grout/README.md — reword the skeleton framing to cover both modes and add:

    ## Watch mode

        go run ./grout watch [-server http://127.0.0.1:8080] [-since 30m] [-tick 60s] [-gate-log PATH]

    Attaches to the agentsview daemon (starting one via `serve --background`
    if none answers), consumes `data_changed` from `/api/v1/events`, and on
    each event — plus every `-tick` — re-lists sessions active in the last
    `-since` via `session list --server … --include-one-shot
    --include-automated --include-children --active-since …`, emitting one
    session row per zellij pipe with fresh `ts`. Reconnects with backoff on
    stream loss (90 s of silence forces it). One refresh in flight at a
    time; a wedged pipe costs ≤5 s per row (kill timer) and never wedges
    grout. Ctrl-C exits cleanly. Grout never stops the shared daemon.

## Acceptance criteria

Offline (agent-reproducible; fake agentsview binary + Go httptest SSE server
+ fake `zellij` argv recorder — the sprint-0 fake discipline):

**AC-1 — Value, wire-level: sessions appear, update, and stop refreshing,
driven by events.** Against a scripted httptest SSE stream and a fake
agentsview whose list responses change across refreshes (fixture cycle: A+B →
A′ with changed state + B → B only), grout watch emits: rows for A and B
after the first refresh (appear); a row carrying A′'s changed state within 5s
of the revealing data_changed's delivery (update); no further A rows after A
leaves the fixture list, while B keeps re-emitting with strictly increasing
`ts` (stop-refresh + the fresh-ts seam hj consumes). Counts and field values
baseline from the scripted fixtures, outside grout's source.
Verified by: `cd grout && go test -run TestWatchSessionsFlow ./...`

**AC-2 — Attach-or-start and reconnect.** With no SSE server listening,
grout watch execs `agentsview serve --background --no-browser
--no-update-check` (fake argv recorded) and connects once the fake server
appears; when the established stream is dropped server-side, grout
reconnects with backoff and runs a full refresh (post-reconnect rows
observed); sustained silence past the threshold (compressed via injectable
threshold) forces reconnect.
Verified by: `cd grout && go test -run 'TestWatchAutoStart|TestWatchReconnect' ./...`

**AC-3 — A wedged rail cannot wedge or flood the watcher.** With the fake
zellij never exiting: refreshes stay single-flight (never more than one live
zellij child at once — recorded pids probed), a burst of data_changed events
during a stuck refresh coalesces to exactly one follow-up refresh, and
SIGTERM ends the process within 10s leaving no children.
Verified by: `cd grout && go test -run TestWatchWedgedPipe ./...`

**AC-4 — Cross-tab bind and click decisions.** Over a synthetic multi-tab
manifest: a unique own-tab cwd match binds to the own-tab pane even when a
foreign tab also matches; own-tab zero + exactly one foreign match binds
cross-tab and the click decision is FocusPane(that pane id); duplicate
matches anywhere → unbound with a dead click. Expected pane ids come from
the constructed manifest fixture.
Verified by: `cargo test bind_cross_tab` (full suite:
`cargo test && cargo check --tests`)

**AC-5 — One-shot mode regression-free.** The existing grout suite passes
unchanged under GOPROXY=off — the watch addition leaves the skeleton's
one-shot behavior and tests untouched.
Verified by: `cd grout && go test ./... && go vet ./...`

Interactive (settled only by CL's live demo):

**AC-6 — Value: the rail fills and stays true without CL running anything,
against an independent baseline.** In CL's fresh zellij session, one
`grout watch` start fills AGENTS within 30s with the same session count
`agentsview session list --include-one-shot --include-automated
--include-children --active-since <horizon> --json` reports at that moment
(count parity — the baseline moves the wrong way if grout drops or ghosts
sessions); a brand-new Claude session started in another tab appears ≤30s
after its first message with no grout interaction (10s coalesce + refresh
headroom over the probe's ≤12s).
Verified by: demo script steps including the count-parity check.

**AC-7 — Cross-tab click lands the operator on the agent.** Clicking a
session row whose bound pane lives in another tab switches the view to that
tab with that pane focused (the 0.44.1 doc contract observed live;
refutation → go_to_tab fallback, back through implementation).
Verified by: demo script; also test-plan item 1, the first live check.

**AC-8 — Dogfood exit: alt-tab scanning replaced.** After a real dogfood
window (a working day with ≥2 concurrent workflows), CL confirms at the gate
that finding the next blocked agent went through the rail, not tab-scanning
— the sprint-1 exit criterion, and it can move the wrong way (CL still
alt-tabs → REJECTED with findings).
Verified by: CL's verdict at the validation gate.

## Test plan

1. **FIRST — cross-tab live check (minutes, piggybacks on any session):**
   two tabs, a terminal pane in tab 2 with a unique cwd, rail visible in
   tab 1; from tab 1 click that session's bound row (or drive
   `focus_terminal_pane` from a scratch keybind pre-wiring); expect the view
   to land on tab 2 with the pane focused, and the bound marker to appear
   for the foreign-tab session (proves foreign-tab `get_pane_cwd` reads).
   Refutes-or-confirms the one remaining unproven mechanism (AC-7); on
   refute, fall back to go_to_tab+focus — an implementation-level change
   only, the bind set already carries the tab.
2. **SSE + enrichment — already proven at ideation** (probe on the record
   above): daemon start/status/stop, `/api/v1/events` data_changed on a
   watched-file write (≤12s), heartbeat frames, list/get `--server`
   enrichment (~60ms), server-side `--active-since`. Re-runnable recipe:
   env-isolated daemon on :18080 (`AGENTSVIEW_DATA_DIR`/`CLAUDE_PROJECTS_DIR`
   at a scratch dir), `curl -N` the events URL, write a synthetic session
   JSONL under `<projects>/<munged-cwd>/<uuid>.jsonl`, `session list
   --server`.
3. **Offline TDD suite** (red-first per the implementation stage):
   TestParseSSEEvents (the probe's recorded frames: data_changed +
   heartbeat), TestWatchSessionsFlow (AC-1), TestWatchAutoStart +
   TestWatchReconnect (AC-2), TestWatchWedgedPipe (AC-3), cargo
   bind_cross_tab + click-decision cases (AC-4). Fakes: httptest SSE server
   (loopback only), fake agentsview/zellij scripts in t.TempDir() — nothing
   binary committed.
4. **Hermeticity:** `go test ./... && go vet ./...` under GOPROXY=off — no
   real agentsview/zellij/daemon; the only sockets are the tests' own
   loopback httptest listeners.
5. **Live demo (validation gate):** spot-check first (test-plan item 1's
   click check plus one `zellij pipe --name agent-event -- ping`), then
   AC-6 fill/parity/appear, AC-7 click, and the dogfood window for AC-8.
   Demo script prepared at validation per workflow rules.

## Out of scope

- Gate glob discovery, folding via the subspace binary, parked/defer rows
  (sprint 2) — watch mode's `-gate-log` stays the skeleton's single-log
  emit.
- `termination_status`→marker vocabulary mapping and the one-shot default
  gate-log path fix (ka, dispatched ahead).
- Row expiry, unbind debounce (hj) — this task only guarantees the
  fresh-ts / stop-refresh seam.
- Verdict actions, `<log>.addr` (M2/sprint 3); multi-machine; `--server`
  auth tokens (single-user local).
- Protocol changes: row kinds/fields and one-row-per-pipe-invocation stay
  as shipped; batching only if dogfood shows pipe-spawn pain.
- Per-session `/api/v1/sessions/{id}/watch` streams; agentsview version
  management beyond recording what the probe ran against.

## Stage Report: ideation

- DONE: Acceptance criteria written as end-state properties with Verified by clauses, split into offline (agent-reproducible) and interactive (CL live-demo) ACs
  AC-1..5 offline (go test / cargo test commands), AC-6..8 interactive; AC-1 and AC-6 measure the end value against fixture and live list-count baselines outside the code under test.
- DONE: Riskiest unproven mechanism named (SSE data_changed consumption + session list/get --server enrichment, cross-tab go_to_tab binding) with the smallest end-to-end check listed first in the test plan
  SSE + enrichment exercised live at ideation against the real v0.36.1 binary (probe evidence in the body: /api/v1/events, data_changed ≤12s, 10s coalesce floor, list --server ~60ms, server-side --active-since); the remaining live-only residue (foreign-tab focus_terminal_pane / get_pane_cwd) is test-plan item 1 with a go_to_tab fallback.
- DONE: Doc diff proposed in the body for the user-visible change (sessions appearing/updating/disappearing without CL running anything)
  README.md agent-rows bullet rewrite + grout/README.md "Watch mode" section; the disappearance seam split honestly against hj (stop-refresh here, visible expiry there).

### Summary

Designed sprint-1's long-running `grout watch`: attach-or-start the
agentsview daemon, consume `/api/v1/events` `data_changed` (proven live
during this ideation with an env-isolated v0.36.1 daemon — event shape, 10s
coalesce floor, heartbeat, ~60ms `list --server` enrichment, server-side
`--active-since` all on the record), full re-emit of the active window per
refresh with fresh `ts` (the expiry seam hj consumes), single-flight
refreshes, reconnect with backoff. Plugin side: binding widens to all tabs
(own-tab-first rule, never guessed); click keeps `focus_terminal_pane`,
whose 0.44.1 contract switches tabs — the one live-only residue, listed
first in the test plan with a `go_to_tab` fallback. Sibling seams to ka
(state vocabulary) and hj (expiry) drawn explicitly.

## Stage Report: implementation

- DONE: Each commit: red test first with the failure reason recorded, then the minimal fix, one behavior per commit
  10 commits (c356a4e…d08d8b5), each red-first. Representative behavioral red — AC-3: `burst coalesced to 3 refreshes, want exactly 2 (one follow-up)` (synchronous loop drained events one-by-one); green after async single-flight + dirty-coalesce. Other reds were compile-fail on the new symbol/signature (`undefined: sseEvent/parseSSEEvents`, `undefined: watchConfig/runWatch`, `unknown field backoffStart`, `undefined: TabPane/terminal_panes_all_tabs` + 12 arity errors). Polling red: `the wedge lands in the foreign pane's backoff` left `None` right `Some(1)`, plus prune red left `{}` right `{20: "/proj"}`.
- DONE: go test ./... + go vet green (grout: watch.go/sse.go/list.go) AND cargo test + cargo check --tests green (plugin: cross-tab binding + click) — both suites
  Go 16/16 test funcs pass under GOPROXY=off, `go vet` clean (also `-race -count=3` clean). Rust 136 tests pass, `cargo check --tests` clean. New files: grout/{sse,list,watch}.go; plugin changes in src/main.rs.
- DONE: Stage report includes before/after test counts and the exact red output; the live cross-tab focus_terminal_pane check (test-plan item 1 / AC-7) is a validation-gate demo, not an implementation-stage requirement — note it as deferred
  Go: 6 → 16 test funcs. Rust: 130 → 136 tests. AC-7 (foreign-tab `focus_terminal_pane`/`get_pane_cwd` live behavior) is DEFERRED to the validation gate — host-call paths (`refresh_statuses` cwd poll) are drillable but the real focus/switch is live-only; not faked offline.

### Summary

Shipped `grout watch` (Go): a pure `parseSSEEvents`, `session list --server`
enrichment, and a single-flight refresh loop that attaches-or-starts the
daemon, consumes `/api/v1/events` `data_changed`, full-re-emits the active
window with fresh `ts`, reconnects with backoff (90s-silence forced), and
exits cleanly on SIGTERM without orphaning a wedged pipe (ctx threaded
through EmitRow). Plugin (Rust): `terminal_panes_all_tabs` widens the bind
domain; `bind_session` is own-tab-first then a lone cross-tab match else
unbound; `decide_rail_click` and the cwd poll extend over the all-tabs set
under the existing wedge/backoff budget. One-shot behavior and its tests are
untouched (AC-5). AC-6/7/8 settle live at the validation gate.

## Stage Report: validation

- DONE: Per-offline-AC verdict (AC-1..5) with independently re-run commands/evidence (both go test and cargo test suites), not re-reading the implementer's stage report
  All 5 PASS, re-run fresh (`-count=1`, no cache) at `d08d8b5`: AC-1 `TestWatchSessionsFlow`, AC-2 `TestWatchAutoStart|TestWatchReconnect` (+bonus `TestWatchReconnectOnSilence`), AC-3 `TestWatchWedgedPipe`, AC-4 `cargo test bind_cross_tab` (4/4) + full `cargo test` (136/136) + `cargo check --tests`, AC-5 `go test ./... && go vet ./...`. Independently recounted 16 Go test funcs / 136 Rust tests (matches stage report). Full table in `docs/agent-rail-dev/.spacedock-state/gates/grout-sse-daemon-validation.md`.
- DONE: Refutation audit executed on a throwaway checkout (never the implementation worktree) naming concrete attack scenarios attempted and why each failed, or a REFUTED with file:line
  Fresh `git clone` + checkout at `d08d8b5` (discarded clean after). 6 attacks run: truncated-final-SSE-frame (real parser-level false negative, `sse.go:27` — but mitigated end-to-end by reconnect-always-refreshes, confirmed via `TestWatchReconnect`'s own reconnect-refresh assertion), malformed session-list JSON (no panic), render line-index bounds (no panic), `"watch"` argv shadowing one-shot's SessionID (no real caller impact — session ids are UUIDs), own-tab-duplicate coverage gap (untested but not a distinct code path), `serve stop` never called. No REFUTED against AC-1..5. Full detail in the gate file above.
- DONE: Demo script prepared for AC-6/AC-7 (interactive) and the AC-8 dogfood-exit framing; subspace review record with decisions logged under docs/agent-rail-dev/.spacedock-state/gates/
  Written to `docs/agent-rail-dev/.spacedock-state/gates/grout-sse-daemon-validation.md`: build/install steps (rebuild required — this task touches `src/main.rs`, unlike the sibling ka task), fresh-session count-parity check, cross-tab click check, and the AC-8 framing question. A live spot-check beyond the prepared script was also run in this review (see below) to de-risk the demo before handing it to CL.

### Summary

Independently re-ran every offline AC (AC-1..5) fresh against `d08d8b5` in the
implementation worktree — all PASS, no reliance on the implementer's reported
numbers. Ran a 6-attack refutation audit on a throwaway clone (never the
worktree): found one real parser-level false negative (a truncated final SSE
frame is silently dropped) but traced it as non-blocking because reconnect
always forces a full refresh, independently confirmed by an existing test. As
an extra spot-check beyond the offline suites, ran `grout watch` against a
real, env-isolated `agentsview` v0.36.1 daemon (not fakes) with a recording
`zellij` stub standing in for the pipe target (no live zellij session was
available in this review's sandbox): a synthetic session appeared within ~1s,
re-emitted with a fresh `ts` every tick, and `session list --server` reported
exact count parity with what grout piped — proving the shipped mechanism
end-to-end ahead of CL's live demo. Wrote the demo script for AC-6/AC-7 and
the AC-8 dogfood framing to the gate file; CL settles those three live — no
fabricated decision log, since only CL's actual review can populate one.
