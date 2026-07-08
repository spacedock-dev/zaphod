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
Plugin side: session→pane binding and rendering both stay scoped to the
rail's own tab — CL structures projects by tab, so a session belongs to
whichever tab's pane cwd it matches, and one matching no pane in this tab
does not render as a row here at all. (Cycle 1 shipped cross-tab binding
and a cross-tab click; cycle 2 reverted both per CL's feedback below.)

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
(subspace recon @9be5fbc). (Cycle 1 feedback dropped cross-tab binding and
click entirely — CL wants projects scoped strictly by tab — so the
foreign-tab `focus_terminal_pane`/`get_pane_cwd` probe above is retained as
historical record only; it no longer gates any acceptance criterion.)

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
  --json --include-one-shot --active-since <now − since>` (shelling out to
  the agentsview binary — the enrichment surface the plan pins, since events
  carry no content; automated and child/subagent sessions stay excluded —
  agentsview's default — so no subagent session ever reaches a row; cycle 2
  dropped `--include-automated --include-children` per CL's feedback that
  subagents must never be listed), decode `{"sessions":[…]}`, build one row
  per entry via the existing `BuildSessionRow`, emit each via the existing
  `EmitRow` (5s kill timer, sequential, never `--plugin` — all shipped
  semantics).
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

### Plugin: same-tab binding + tab-scoped rows

CL's cycle-1 feedback: "I want the sidebar's agent/pane/gate bound to panes
of the same tab, so that I can structure projects by tab." Cycle 2 reverted
cross-tab binding/click entirely rather than adjusting it.

- **Binding domain stays the rail's own tab** — the shipped
  `rows_for_own_tab` pane set (!is_plugin, is_selectable, !is_suppressed):
  `bind_session` never looks at another tab's panes. The cycle-1
  `terminal_panes_all_tabs(manifest)` widening, its `TabPane` type, and the
  foreign-tab cwd-poll extension are removed — not merely unused.
- **Bind rule (`bind_session`):** exactly one own-tab cwd match → bind
  there; zero or two-plus matches → unbound (never guessed). No fallback to
  any other tab.
- **Scope filter (new, `sessions_in_own_tab`):** a session whose cwd matches
  no own-tab pane is filtered out of the AGENTS section entirely, before
  both render and click decisions — it does not appear as an unbound row,
  it does not appear at all. This is a strictly looser test than binding:
  "in scope" needs at least one own-tab cwd match; "bound" needs exactly
  one, so an in-scope-but-ambiguous session still renders, unbound.
- **Click (`decide_rail_click`):** a bound row click stays
  `ClickAction::FocusPane(id)` → `focus_terminal_pane(id, false, false)`,
  always within the rail's own tab. There is no cross-tab click and no
  `go_to_tab` fallback — that mechanism is dropped along with the binding
  it served. Nav-mode exit on click (F8 fix) already shipped.
- Rendering unchanged otherwise: bound/unbound marker as shipped; state
  markers are ka's.

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

### Doc diff (applied, revised cycle 2)

README.md, the Agent & gate rows bullet:

    - **Agent & gate rows**: a companion `grout watch` daemon streams
      `agent-event` rows into the rail — every agent session active in the
      last 30 minutes and belonging to the tab's own project appears with its
      state, updates as agentsview sees new activity (≈10 s coalescing), and
      stops refreshing once it leaves the window (visible row expiry lands
      with the row-lifecycle task). Binding never crosses tabs: a session
      outside the tab's own project scope, or a subagent/automated session,
      never appears as a row in any tab. Click a session row to focus its
      cwd-bound pane; click a gate row to float `subspace-tui` on the gate's
      artifact with `--log` pointed at its decision log. Requires the
      `RunCommands` permission (prompted once)

grout/README.md — reword the skeleton framing to cover both modes and add:

    ## Watch mode

        go run ./grout watch [-server http://127.0.0.1:8080] [-since 30m] [-tick 60s] [-gate-log PATH]

    Attaches to the agentsview daemon (starting one via `serve --background`
    if none answers), consumes `data_changed` from `/api/v1/events`, and on
    each event — plus every `-tick` — re-lists sessions active in the last
    `-since` via `session list --server … --include-one-shot
    --active-since …`, emitting one session row per zellij pipe with fresh
    `ts`. Automated and child (subagent) sessions stay excluded —
    agentsview's default — so no subagent ever reaches a row; one-shot
    sessions stay included as a distinct top-level invocation. Reconnects
    with backoff on stream loss (90 s of silence forces it). One refresh in
    flight at a time; a wedged pipe costs ≤5 s per row (kill timer) and
    never wedges grout. Ctrl-C exits cleanly. Grout never stops the shared
    daemon.

(Cycle 1 proposed and applied a cross-tab-binding version of the README
bullet, with `--include-automated --include-children` on the list command;
cycle 2 replaced both with the text above per CL's feedback below.)

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

**AC-4 — Same-tab-only bind decisions; out-of-scope sessions never render.**
A session whose cwd matches no pane in the rail's own tab is out of scope:
it does not bind — even when a foreign tab's pane shares the cwd, and even
though cycle 1's design would have bound it cross-tab — and it is filtered
out of the AGENTS section entirely, not merely rendered unbound. Zero
cross-tab rows and zero cross-tab clicks exist anywhere in the fixture. A
unique own-tab cwd match still binds and its click decides
`FocusPane(that pane id)`; two own-tab panes sharing a cwd keep the session
in scope but unbound (dead click).
Verified by: `cargo test -- bind_never_crosses_tabs sessions_outside_own_tab_scope_are_filtered_entirely out_of_scope_sessions_do_not_occupy_a_click_line` (full suite: `cargo test && cargo check --tests`)

**AC-5 — One-shot mode regression-free.** The existing grout suite passes
unchanged under GOPROXY=off — the watch addition leaves the skeleton's
one-shot behavior and tests untouched.
Verified by: `cd grout && go test ./... && go vet ./...`

Interactive (settled only by CL's live demo):

**AC-6 — Value: the rail fills and stays true without CL running anything,
against an independent baseline scoped to the tab's own project.** In CL's
fresh zellij session, one `grout watch` start fills the demo tab's AGENTS
section within 30s with the same session count that `agentsview session
list --include-one-shot --active-since <horizon> --json` reports, filtered
to sessions whose cwd matches one of the demo tab's own pane cwds
(automated/child sessions are excluded from both sides by omitting
`--include-automated --include-children` — agentsview's default, so a
subagent can never inflate either count); count parity — the baseline moves
the wrong way if grout drops, ghosts, wrongly includes an out-of-scope
session, or wrongly includes a subagent session. A brand-new Claude session
started in another tab's project does not appear in the demo tab at all; one
started in the demo tab's own project appears ≤30s after its first message
with no grout interaction (10s coalesce + refresh headroom over the probe's
≤12s).
Verified by: demo script steps including the scoped count-parity check.

**AC-7 — Superseded by AC-4.** Cross-tab click is out of scope: binding and
rendering never cross tabs (see AC-4). There is no cross-tab
`focus_terminal_pane` behavior left to confirm live — the out-of-scope-row
assertion is AC-4's offline, agent-reproducible test, not a demo-gated live
check. Do not re-add a cross-tab-click AC.
Verified by: n/a — folded into AC-4.

**AC-8 — Dogfood exit: alt-tab scanning replaced.** After a real dogfood
window (a working day with ≥2 concurrent workflows), CL confirms at the gate
that finding the next blocked agent went through the rail, not tab-scanning
— the sprint-1 exit criterion, and it can move the wrong way (CL still
alt-tabs → REJECTED with findings).
Verified by: CL's verdict at the validation gate.

## Test plan

1. **SSE + enrichment — already proven at ideation** (probe on the record
   above): daemon start/status/stop, `/api/v1/events` data_changed on a
   watched-file write (≤12s), heartbeat frames, list/get `--server`
   enrichment (~60ms), server-side `--active-since`. Re-runnable recipe:
   env-isolated daemon on :18080 (`AGENTSVIEW_DATA_DIR`/`CLAUDE_PROJECTS_DIR`
   at a scratch dir), `curl -N` the events URL, write a synthetic session
   JSONL under `<projects>/<munged-cwd>/<uuid>.jsonl`, `session list
   --server`.
2. **Offline TDD suite** (red-first per the implementation stage):
   TestParseSSEEvents (the probe's recorded frames: data_changed +
   heartbeat), TestWatchSessionsFlow (AC-1), TestWatchAutoStart +
   TestWatchReconnect (AC-2), TestWatchWedgedPipe (AC-3), cargo
   `bind_never_crosses_tabs` + `sessions_outside_own_tab_scope_are_filtered_entirely`
   + `out_of_scope_sessions_do_not_occupy_a_click_line` (AC-4). Fakes:
   httptest SSE server (loopback only), fake agentsview/zellij scripts in
   t.TempDir() — nothing binary committed.
3. **Hermeticity:** `go test ./... && go vet ./...` under GOPROXY=off — no
   real agentsview/zellij/daemon; the only sockets are the tests' own
   loopback httptest listeners.
4. **Live demo (validation gate):** the scoped AC-6 fill/parity/appear check
   and the AC-8 dogfood window. (Cycle 1's cross-tab click live check —
   formerly item 1 here, gating the now-superseded AC-7 — no longer
   applies: there is no cross-tab mechanism left to confirm live.)
   Demo script prepared at validation per workflow rules.

## Out of scope

- Cross-tab session binding and cross-tab click (dropped per cycle 1
  feedback — CL wants projects scoped strictly by tab); `go_to_tab` and
  foreign-tab `focus_terminal_pane`/`get_pane_cwd` are not used anywhere in
  this task.
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

### Feedback Cycles

**Cycle 1 (2026-07-08) — REJECTED at validation, routed to implementation.**

CL's live demo (2026-07-08) surfaced a real spec mismatch, independent of a
build mixup along the way (Tab #6's rail was loaded from the main-repo wasm,
not this worktree's build — the "AC-7 doesn't switch tabs" symptom CL first
saw was that mixup, not this finding; re-confirmed the real gap directly
from CL's stated requirements once corrected):

1. **AC-4/AC-7 direction is wrong.** The implemented design
   (`terminal_panes_all_tabs`, `bind_session`'s "own-tab-first, then a lone
   cross-tab match, else unbound," `ClickAction::FocusPane` switching tabs)
   is exactly what AC-4/AC-7 asked for — but CL does not want cross-tab
   binding or cross-tab click at all. CL's words: "I want the sidebar's
   agent/pane/gate bound to panes of the same tab, so that I can structure
   projects by tab." Fix: revert to (or design fresh) strict same-tab-only
   binding — a session whose cwd doesn't match a pane in the sidebar's own
   tab is out of scope, not "unbound" — see point 2, it should not appear
   as a row at all, not merely be unclickable.
2. **Out-of-scope sessions must not be listed, not just be unclickable.**
   Confirmed via explicit follow-up (CL chose "filter out entirely" over
   "list but mark unbound"): a tab's AGENTS section must show only sessions
   whose cwd falls under that tab's own project scope (matched against the
   panes/cwds already visible to that tab's sidebar instance). Sessions
   outside that scope should not be emitted as rows in that tab at all —
   this is a real filtering change (likely in the plugin's session-render
   path, matching `self.pane_cwds`/`self.rows` for the tab), not a label
   change.
3. **Subagent/automated/child sessions must never be listed, in any tab.**
   CL: "we shouldn't list subagents." The current design queries
   `agentsview` with `--include-automated --include-children` (per AC-6's
   own baseline command) and grout emits whatever it gets back undifferentiated.
   Fix: exclude automated/child (subagent) sessions from what grout emits,
   or from what the plugin renders — implementation decides which layer is
   the right place, but the end state is that no subagent session ever
   appears as a row, in any tab.

**Revisions needed to this entity's own ACs, not just the code:**
- **AC-4** must change from "cross-tab bind and click decisions" to
  "same-tab-only bind decisions: a session whose cwd doesn't match a pane
  in the given tab's own set is out of scope, not bound, and not rendered
  as a row" — the synthetic multi-tab fixture test should assert zero
  cross-tab rows/clicks, not a cross-tab FocusPane decision.
- **AC-6**'s baseline changes from system-wide
  `agentsview session list --include-one-shot --include-automated
  --include-children` count parity to: count parity against sessions (a)
  whose cwd falls under the demo tab's own project scope and (b) excluding
  automated/child sessions — restate the exact baseline command implementation
  lands on.
- **AC-7** is dropped/superseded — replaced by an assertion that a session
  outside the current tab's scope never appears as a row (already covered
  by the revised AC-4's rendering-side assertion); do not re-add a
  cross-tab-click AC.
- **AC-8** unaffected.

Routed to implementation fresh (no addressable worker handle from this
session for the prior implementation/validation ensigns).

## Stage Report: implementation (cycle 2)

- DONE: Revert/replace cross-tab binding: bind_session (and terminal_panes_all_tabs's call sites) must only match sessions against panes in the sidebar's own tab; drop the "own-tab-first, then a lone cross-tab match" fallback and the cross-tab FocusPane click entirely, per Feedback Cycles cycle 1.
  Commit 2a7b8c6. Red: rewrote the bind_session/decide_rail_click test call sites to the 3-arg/5-arg (no all_panes) shape while the source still had the 4-/6-arg cross-tab signature — `error[E0061]: this function takes 4 arguments but 3 arguments were supplied` (bind_session) across ~6 call sites. Green after reverting bind_session/decide_rail_click to own-tab-only, deleting TabPane/terminal_panes_all_tabs/all_tab_panes and the foreign-tab cwd-poll extension in refresh_statuses. New test `bind_never_crosses_tabs` pins a lone foreign-tab cwd match staying unbound with a dead click.
- DONE: Filter sessions outside a tab's own project scope (cwd not matching one of that tab's own pane cwds) out of the emitted/rendered rows entirely for that tab -- not merely labeled unbound -- and exclude automated/child (subagent) sessions from what grout emits or the plugin renders, in every tab.
  Two commits. Plugin (3d5abbd): red — `error[E0425]: cannot find function 'sessions_in_own_tab'` for two new tests; green after adding `sessions_in_own_tab` (in-scope test: any own-tab cwd match, looser than bind_session's exactly-one) and wiring it into render() and handle_click() ahead of section_layout/decide_rail_click. Grout (23ccd7d): red — `TestListSessionsArgv` asserted the argv without `--include-automated --include-children` against the still-flagged `listSessions`, failing on the recorded argv mismatch; green after dropping both flags (agentsview's own default already excludes automated/child sessions; `--include-one-shot` stays since a one-shot run is a distinct top-level session, not a subagent).
- DONE: Update AC-4 (same-tab-only bind decisions, asserting zero cross-tab rows/clicks), AC-6 (baseline restated for tab-scoped, non-subagent count parity), and AC-7 (drop cross-tab click, replace with an assertion that out-of-scope sessions never render as rows) to match, with red-test-first evidence for each changed behavior.
  AC-4/6/7 rewritten in this entity's Acceptance criteria section above (AC-7 marked superseded by AC-4, kept as a numbered placeholder per feedback rather than renumbering AC-8). Test plan's former item 1 (the cross-tab live check gating the old AC-7) removed and the list renumbered; the "Plugin: cross-tab binding + click" and "Doc diff" design sections rewritten to match the shipped same-tab design; README.md and grout/README.md updated (commit 8b04a4f) since they still described cross-tab switching and the automated/children flags. Red/green evidence for the underlying behavior changes is the two items above; this item is documentation, not a new test.

### Summary

Reverted cycle 1's cross-tab session binding entirely per CL's cycle-1
feedback (own-tab-only again: `bind_session`/`decide_rail_click` back to
their pre-cross-tab signatures, `TabPane`/`terminal_panes_all_tabs`/
`all_tab_panes` and the foreign-tab cwd-poll deleted, not merely disabled),
then added the two behaviors cycle 1 was missing: `sessions_in_own_tab`
filters a tab's AGENTS section to sessions whose cwd matches at least one
own-tab pane (out-of-scope sessions render nowhere, not just unbound), and
grout's `listSessions` dropped `--include-automated --include-children` so
agentsview's own default keeps subagent/automated sessions out of every row
grout ever emits. Four commits, each red-first (`cargo test`/`go test`
failures recorded above); `cargo test` (133/133) + `cargo check --tests`
and `go test ./... && go vet ./...` (GOPROXY=off) both green. Rust test
count: 136 → 133 (net −3: six cycle-1 cross-tab tests removed, three
same-tab/scope tests added). Go test count unchanged at 16 (only an
existing test's expected argv changed). Updated AC-4/6/7, the test plan,
the design narrative, and both READMEs to match; AC-1/2/3/5/8 untouched.

## Stage Report: validation (cycle 2)

- DONE: Independently re-run AC-1/2/3/5 (unaffected by cycle 2) and AC-4 (rewritten: same-tab-only bind decisions, zero cross-tab rows/clicks) against the worktree's cycle-2 commit, re-executed not re-read
  All 5 PASS, fresh `-count=1` at `8b04a4f`: AC-1 `TestWatchSessionsFlow`, AC-2 `TestWatchAutoStart|TestWatchReconnect`(+bonus silence test), AC-3 `TestWatchWedgedPipe`, AC-4 the 3 named cargo tests + full `cargo test` (133/133) + `cargo check --tests`, AC-5 `go test ./... && go vet ./...`. Full table in `docs/agent-rail-dev/.spacedock-state/gates/grout-sse-daemon-validation.md` under "Cycle 2".
- DONE: Refutation audit on a throwaway checkout (never the implementation worktree) targeting same-tab-only filtering and the dropped --include-automated/--include-children flags, naming concrete attacks and documenting survivors or REFUTED with file:line
  Fresh `git clone` + `checkout 8b04a4f` (discarded after). Cross-tab click/bind REFUTED as structurally impossible (`bind_session`/`decide_rail_click` only ever receive tab-scoped `rows_for_own_tab` output, main.rs:542/2038/2026 — no foreign-tab pane id can reach them). Shared-cwd-across-two-tabs SURVIVES (reproduced with a throwaway test): the same session binds independently in two tabs that happen to share a pane cwd — not a code defect (no cross-tab data flow), but a named residual scope ambiguity for CL's awareness. Subagent-leak-via-one-shot-mode's `session get` path considered and set aside (different threat model, out of scope per AC-5). No redundant automated/child check exists anywhere outside the omitted argv (confirmed by grep — single point of enforcement). Empirical probe of agentsview's own child-session classifier against a real env-isolated v0.36.1 daemon was INCONCLUSIVE — my synthetic sidechain-shaped session wasn't recognized as a child session at all, and this sandbox can't read agentsview's source to confirm the true mechanism; flagged honestly rather than claimed as verified. Full detail in the gate file's "Cycle 2" refutation section.
- DONE: Revised demo script for AC-6's interactive half (tab-scoped, non-subagent count parity; unattended new-session appearance), AC-7 dropped with no cross-tab click step, AC-8 kept separate
  Written to the gate file's "Cycle 2" section: scoped baseline command (per-own-tab-pane-cwd `jq` filter, no `--include-automated`/`--include-children`), fill+parity check, unattended same-tab appearance check, and a negative check that an out-of-scope tab's new session never appears — no cross-tab click step anywhere in this script.

### Summary

Independently re-ran AC-1..5 fresh against cycle-2 commit `8b04a4f` — all PASS,
matching the implementer's counts (Rust 133/133, Go 16 funcs) without trusting
them. Ran a refutation audit on a throwaway clone (discarded after) targeting
exactly the two cycle-2 behavior changes: proved cross-tab bind/click is
structurally impossible by tracing the tab-scoped data flow, but found and
named one real residual (two tabs sharing a pane cwd double-list the same
session — not a defect, a design edge case) and one honest verification gap
(couldn't confirm agentsview's own automated/child classifier from this
sandbox — no source access, and my synthetic subagent fixture didn't trigger
its classification at all). Rewrote the AC-6 demo script for tab-scoped,
non-subagent count parity with an unattended-appearance check and an
out-of-scope negative check; dropped the cross-tab click step entirely since
AC-7 is superseded. AC-8 stays a separate dogfood-window question for CL.
