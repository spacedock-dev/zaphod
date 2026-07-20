---
id: 91f2dxkn3v7fe1174ayj48j5
title: Remove synchronous pane metadata calls from the plugin hot path
status: validation
source: live nautical-cuckoo congestion diagnosis 2026-07-14
sprint: s1-managed-tab-safety
group: architecture-hardening
sprint-readiness: ready
started: 2026-07-14T11:26:05Z
completed:
verdict:
score: 0.98
worktree: .worktrees/spacedock-ensign-nonblocking-pane-metadata-architecture
issue:
pr:
mod-block:
---

## Problem

The WASM event/timer loop synchronously enriches terminal rows through pane command, CWD, and scrollback host calls. Even after the five-second scrollback hotfix, any slow host call can couple sidebar refresh to Zellij's shared runtime queues and make unrelated input or layout actions stall.

Task `44` removed periodic scrollback and restored the narrow usable path. It
did not remove `get_pane_running_command` or `get_pane_cwd`; one visible rail
still invokes both for each eligible terminal every two seconds. Backoff limits
retries only after a call returns. It cannot stop the current plugin handler
from blocking, cancel obsolete work, or prevent several rails from multiplying
load.

The accepted KJ design also changes the authority boundary. A SessionStart
hook and the tab's manual watcher identify an agent session's exact Zellij pane. CWD,
the foreground command, titles, prompt text, and temporal proximity no longer
authorize session binding. Task 91 must use that exact identity rather than
building a second pane-discovery mechanism.

## Required outcome

Make the plugin render from bounded cached/event-fed state only. Slow pane enrichment must run outside the WASM hot path with explicit deadlines, limited concurrency, cache/backoff, cancellation, and stale-state presentation. Exact pane identity remains authoritative; unavailable enrichment renders degraded or unbound rather than guessing. Add a congestion regression that exercises a slow/non-shell pane while repeated pane/tab actions remain prompt and do not burst later.

Implementation starts from a mainline that contains task `44` and KJ's
approved exact session-to-pane projection. If KJ has not merged, stop; do not
restore CWD binding or add a shared registry to make this task independently
green.

## Riskiest mechanism spike — run first, PASSED

The riskiest mechanism is cancellation and queue ownership, not map lookup.
The smallest spike therefore kept slow work in a native process that never
called Zellij, coalesced 100 revisions for one pane to the latest revision,
allowed one worker, and held revision 100 in flight until a separate native
observer released it.

On main `5677ef6`, Zellij 0.44.3, the disposable session performed these
operations while a six-second enrichment remained in flight:

```text
non-shell pane: 22 ms
new pane 1:     22 ms
new pane 2:     21 ms
new pane 3:     21 ms
new tab:        24 ms
go to tab 1:    19 ms
go to tab 2:    19 ms
observed:       6 terminals, 2 tabs
```

The observer captured pane and tab state before releasing the barrier. The
scheduler then canceled the work and waited beyond the original six-second
deadline. It reported `latest-only=100`, `max_active=1`, and
`late_publications=0`; the complete pane lifecycle tuple and active-tab
projection remained unchanged. The probe used isolated data and socket roots
and deleted its session. It did not read or write standing Zellij KDL.

This spike validates process isolation, latest-wins coalescing, bounded
concurrency, cancellation, and the absence of a delayed burst. It does not
validate the production pipe protocol or WASM cache. The first implementation
test below repeats the same barrier through the candidate watcher, private
pipe, plugin cache, and literal-key path.

## Proposed approach

### 1. Make `PaneUpdate` the only pane-state authority in WASM

Keep `rows_for_own_tab` as the pure projection of the current manifest. Extend
it to derive only data already present in `PaneInfo`: pane ID, title, focus,
visibility, lifecycle fields, and command-pane launch metadata when Zellij
supplies it in the event. Keep `registered_session_pane` from KJ as the only
session-to-terminal join.

Delete periodic calls to `get_pane_running_command`, `get_pane_cwd`, and
`get_pane_scrollback`. Remove their backoff maps, CWD cache, host seam, and
`ReadPaneContents` permission. A timer may compare cache age and schedule a
render; it must not call a pane metadata API.

An unmanaged terminal starts as `unknown . unknown` unless its manifest title
or command-pane launch record identifies a provider. A registered session gets
state and summary from the event feed. Missing enrichment never removes the
terminal row and never enables focus for a session without exact registered
pane membership.

### 2. Feed one atomic, bounded per-tab cache over the existing private channel

Use the KJ manual watcher as the only native session feed. The operator starts
one watcher from the selected terminal in each direct-entry tab. Its one
startup inventory proves the exact Zellij session, stable tab, terminal pane,
original rail, recipient token, and socket root; after readiness it performs
no native pane inventory or pane-metadata lookup. A trusted matching
`SessionStart` supplies one top-level provider registration through that
terminal's private socket. The watcher keeps that registration only in memory,
and a later matching `SessionStart` replaces it.

Each accepted full snapshot carries:

```text
stream_generation, snapshot_seq, observed_at, source_health,
sessions[{provider_session_id, pane_id, agent, state, summary, updated_at}]
```

The SessionStart registration and exact AgentsView ID remain authoritative.
Each watcher fetches only `/api/v1/sessions/{provider:id}` for its in-memory
registration and sends its own bounded snapshot to the original
token/tab/rail tuple. It never consults the global session list. The plugin
intersects that snapshot with its current `PaneUpdate`, so only the rail whose
manifest contains the exact registered pane ID renders or focuses the session.
A pane move therefore changes projection on the next `PaneUpdate` without a
CWD lookup or native inventory poll.

Validate a full snapshot atomically before replacing the cache. Reject the
whole record on a duplicate session ID, duplicate pane ID, noncanonical pane
ID, invalid state, malformed UTF-8, trailing JSON, or a payload beyond KJ's
existing 1 MiB registry/session envelope. Retain the last good cache and mark
its source unhealthy.

Persist only a monotonic `stream_generation` for each recipient under the
existing runtime lock; never persist session authority. Within one generation,
accept only increasing `snapshot_seq`. A restarted watcher advances the
generation, completes a private ready handshake, and publishes an empty
snapshot. Only a fresh matching `SessionStart` may repopulate it. The plugin
rejects lower generations and old or repeated sequences, so a canceled pipe
cannot overwrite newer state if Zellij delivers it late.

### 3. Bound, cancel, and coalesce native enrichment

Add one latest-wins refresh coordinator to each Go manual watcher. Keep
SessionStart ingestion, exact-ID fetching, and `BuildRegisteredSessionRow`
separate from WASM's final manifest intersection. Do not create a shared
registry, persistent recovery controller, or another binding model.

- In-memory registration changes, AgentsView `data_changed`, and the bounded
  recovery timer request a refresh generation. A newer request cancels the
  current generation and replaces the single pending generation.
- At most two exact-ID HTTP enrichments run at once. Each gets a 500 ms request
  deadline and the parent generation's context. Results from a canceled or
  superseded generation are discarded.
- Successful records use a 2-second freshness TTL. Timeouts and source errors
  retain the last good value, mark it stale, and back off per session at 1, 2,
  4, 8, then 30 seconds. A successful exact-ID fetch resets backoff.
- One publisher may run at once. It sends only a complete latest snapshot
  through the private token-bound pipe with a one-second deadline. One pending
  snapshot replaces any older pending snapshot. Cancellation terminates and
  reaps the CLI child.
- No enrichment worker invokes Zellij. Only the bounded publisher does, after
  it has a complete immutable snapshot. No canceled worker may enqueue a
  publish.

These bounds apply per watcher. They cannot congest Zellij's pane metadata
path because they never enter it. The private publisher remains bounded and
coalesced, so a slow rail cannot accumulate a later delivery burst.

### 4. Render fresh, stale, and degraded states explicitly

Keep `apply_agent_snapshot` as an atomic replacement and extend it to return a
validated cache generation. Add a pure projection that combines
`rows_for_own_tab`, the latest accepted snapshot, and cache health.

- Fresh exact registration: render the session and enable focus for its live
  pane.
- Stale exact registration: keep the last session row, add a visible `stale`
  status, and keep focus enabled only while the exact pane remains in the
  current manifest.
- Missing, duplicate, foreign, closed, suppressed, or unselectable pane:
  render no session row and enable no focus action.
- Missing enrichment for an ordinary pane: render the pane with event-derived
  title and `unknown` metadata.
- Watcher loss: the plugin's cache-age timer marks the last snapshot stale;
  it does not clear terminal rows or perform a recovery host call.

Pane focus and layout actions continue to use current manifest IDs and the
existing managed-tab authorization. This task changes metadata acquisition,
not toggle semantics.

## Acceptance criteria

### Offline (agent-reproducible)

**AC-O1 — slow enrichment cannot delay or burst pane/tab actions.** With the
candidate rail loaded, an exact slow non-shell pane has one six-second native
enrichment generation held in flight. Three literal `Alt p` actions, one
literal `Alt n`, and literal `Alt 1` then `Alt 2` each reach the required
native pane/tab state within one second measured from before key delivery.
The barrier remains in flight through the post-action native observations.
Canceling it and waiting six seconds produces no delayed pane, tab, focus, or
snapshot delivery.

Verified by: the isolated tmux/Zellij harness, a watcher debug barrier, and
task 44's structured identity format in
`tests/zellij-pane-metadata-congestion-test.sh`. Each action compares the
complete relevant native record, including pane ID, kind, tab ID, plugin URL, exited,
exit status, held, floating, suppressed, selectable, and the action's explicit
focus/active allowances. The harness records exact event, plugin, refresh ID,
and numeric `pane_ids`; wrong-pane, wrong-field, early-release, post-send
deadline, and late-publication variants must fail.

**AC-O2 — WASM performs no synchronous pane metadata lookup.** Timer,
`PaneUpdate`, render, click, key, and pipe handlers derive pane rows and
session focus from `PaneInfo` plus the last accepted snapshot. They never call
the command, CWD, or scrollback host APIs, and the candidate no longer requests
`ReadPaneContents`.

Verified by: the executable `src/main.rs` source/permission assertion that the
runtime contains zero forbidden host calls and requests no pane-content
permission, plus the timer, `PaneUpdate`, render, click, key, and pipe handler
matrices that assert exact rows, focus decisions, and fail-closed outcomes.
Independent validation must exercise every handler path; no proposed panicking
host seam is required.

**AC-O3 — scheduling, cancellation, and cache size stay bounded.** A burst of
100 refresh requests for one pane produces one latest completed generation,
never exceeds two enrichment workers or one publisher, and retains at most one
pending refresh and one pending snapshot. Closing the pane, canceling the
watcher, or superseding the generation reaps every worker and pipe child. No
result or publish appears after its deadline or cancellation.

Verified by: a deterministic Go coordinator test in
`grout/metadata_scheduler_test.go` with blocked workers, explicit
start/cancel/release barriers, runtime worker/publisher gauges, and a
post-deadline quiet window. Adjacent tests cover two panes, out-of-order
completion, source timeout, pipe timeout, repeated cancellation, and a worker
that ignores cancellation until its deadline.

**AC-O4 — exact per-tab pane authority fails closed across lifecycle and
restart.** Two same-CWD managed tabs each run one manually started watcher and
each render exactly the one top-level session registered from its watched
terminal; a child and unrelated history render nowhere, for exact counts
`1/1/0`. Moving the exact pane out of its watched tab removes the projection
without transferring watcher authority; closing, suppressing, or making it
unselectable also removes the row. Watcher restart
publishes an empty projection and remains empty until a fresh matching
`SessionStart`; that later delivery restores exactly one row. CWD, command,
title, time, row order, and a foreign watcher sharing the recipient token
cannot create a binding.

Verified by: `tests/zellij-two-rail-recipient-smoke-test.sh`, which starts one
watcher per exact same-CWD terminal, attacks stable-tab admission with a shared
token, proves `1/1/0`, exact-row click focus, stale retention, restart-empty,
fresh SessionStart recovery, manifest fail-close, bounded exact-ID fetching,
zero idle native polls, and no durable session-authority record.

**AC-O5 — stale and malformed data degrade without lying.** A source timeout,
watcher kill, malformed snapshot, duplicate identity, over-limit snapshot, old
stream generation, or repeated/out-of-order sequence preserves the last good
terminal inventory. Exact cached sessions become visibly stale after the
configured TTL; absent or ambiguous pane membership disables focus. A later
fresh complete snapshot clears stale state once and never replays discarded
updates.

Verified by: Rust atomic-validator/projection matrices and Go outage/recovery
tests in `src/main.rs` and `grout/metadata_scheduler_test.go` using an
independent protocol fixture. Tests assert the complete
rendered rows, focus actions, retained generation, error attribution, and
recovery state for empty, maximum, maximum-plus-one, malformed, repeated, and
out-of-order records.

**AC-O6 — failure and cleanup remain isolated.** A missed action deadline,
worker leak, failed native observation, source hang, pipe hang, or interrupted
harness fails visibly and removes only its disposable Zellij session, tmux
server, watcher, workers, fixtures, and temporary roots. Standing config and
layout bytes remain unchanged.

Verified by: injected failures under the existing cleanup traps in
`tests/zellij-pane-metadata-congestion-test.sh`, bounded PID and session-absence
checks, pre/post standing-file digests, and retained phase
evidence. A missing dependency fails the required case instead of reporting a
skipped pass.

### Interactive (only after AC-O1 through AC-O6)

**AC-I1 — the captain can work normally beside a slow pane.** In a fresh
managed tab using main-built WASM, the captain manually starts `watch-tab` in
the selected terminal, then opens real `subspace-tui` and a real registered
Codex session. The rail shows exact session state or a visible stale marker.
Three `Alt p` presses, `Alt n`, `Alt 1`, and `Alt 2` each take effect in under
one second; closing the exact sidebar and waiting six seconds causes no delayed
action burst.

Verified by: captain observation plus external native pane/tab snapshots,
watcher gauges, exact refresh records, and standing-KDL hashes. The drill uses
a fresh managed tab and follows `docs/zellij-agentsview-live-demo.md`; it never
retrofits or hot-reloads an existing rail.

## Test plan

1. **Repeat the invalidating spike first through the production seam.** Port
   the passed native scheduler spike into a committed deterministic test. Add
   the manual watcher's barrier and full snapshot, load the candidate WASM in
   the isolated tmux harness, and run AC-O1 through the fixed 28-column rail.
   Prove that the delivered row focuses its exact pane before the one-second
   deadline while enrichment remains held, and that releasing the barrier
   before native observation makes the test fail. If literal actions still
   miss one second, preserve the structured evidence and stop; do not add a
   longer timeout or another polling layer.
2. Rebase or merge only after KJ's exact registration work and task 44 are on
   the implementation base. Resolve the combined source so `registered_session_pane`
   replaces CWD binding and task 44's zero-scrollback rule remains intact.
3. Remove the WASM host calls and permissions. Extend `rows_for_own_tab`,
   `apply_agent_snapshot`, and the session projection as pure functions. Run
   the executable zero-forbidden-call source/permission assertion and every
   handler matrix, then `cargo test` and `cargo check --tests`. Independent
   validation repeats the handler-path proof without requiring a panicking
   host seam.
4. Add the Go latest-wins coordinator to the manual watcher around its single
   in-memory registration, exact-ID fetch, and `BuildRegisteredSessionRow`.
   Prove deadlines, two-worker/one-publisher limits, coalescing, cancellation,
   backoff, stream generation, atomic size limits, and child cleanup without a
   shared registry, durable session authority, or native inventory
   polling/retry.
5. Extend the KJ two-tab harness to start one watcher per same-CWD terminal and
   prove exact per-tab snapshots, move/close/suppress, stale retention,
   restart-empty, fresh SessionStart recovery, and `1/1/0` cardinality. Add
   adjacent foreign-token/tab/rail, wrong-pane, wrong-field, old-sequence,
   duplicate, EOF, Unicode, and maximum-plus-one variants.
6. Run the retained Rust, Go, artifact, new-tab, managed-tab, recipient,
   lifecycle, and layout suites. Run AC-O1's success and injected-failure
   cleanup modes from a fresh build. Verify `git diff --check` and standing KDL
   hashes.
7. Only after the offline packet passes, give the captain AC-I1. A live miss
   blocks validation and preserves evidence; it does not authorize in-session
   retrofit, automatic watcher setup, a shared recovery registry, or broader
   workspace-hub work.

## Documentation change

- Update README's Features, Permissions, Status, and Development sections:
  the rail uses manifest and exact registered-session snapshots, never
  periodic command/CWD/scrollback calls; stale and unknown states are visible;
  `ReadPaneContents` is no longer required. Document one manually started
  watcher per tab, one in-memory registration, restart-empty, and recovery only
  after a fresh matching `SessionStart`.
- Update SPEC's historical status note and `docs/docking-approach.md` to retire
  all periodic WASM metadata calls, not only scrollback, while preserving the
  old prototype findings as history.
- Extend `docs/zellij-tmux-smoke-harness.md` with the external refresh barrier,
  full lifecycle tuples, early-release negative, worker/publisher bounds,
  per-tab `1/1/0`, restart-empty/fresh-registration recovery, and post-cancel
  quiet window.
- Update `docs/zellij-agentsview-live-demo.md` to keep exact registration as
  authority and demonstrate manual watcher launch, visible stale retention,
  restart-empty, and recovery only from a new SessionStart.

## Out of scope

The exact-plugin close/rebuild/reload operator workflow is tracked by
`managed-main-wasm-reload-loop`. This task does not build the evergreen
workspace hub or controller, add non-Codex provider registration, infer a
session without an authoritative hook ID, scrape PTYs or process trees, restore
viewport summaries, poll global pane inventory for enrichment, change managed
toggle/layout semantics, retrofit a foreign tab, mutate standing Zellij KDL,
or redefine KJ's registry authority and session-incarnation decision.

### Feedback Cycles

#### Inherited proof requirements from tactical hotfix 44 — 2026-07-14

- Reuse task 44's structured refresh authority: exact event, plugin, refresh
  ID, and numeric `pane_ids`; do not regress to whole-screen or broad log
  matching.
- Action-preservation evidence must compare the complete relevant terminal
  lifecycle state, including exited, suppressed, floating, and selectable
  fields, with adjacent wrong-pane and wrong-field attacks.
- The congestion proof must hold deterministic slow refresh work in flight
  from before key delivery through native post-action observation. Releasing
  the barrier before observation must fail the test.
- These requirements strengthen the permanent nonblocking architecture; they
  do not expand task 44 beyond removal of periodic scrollback.

#### KJ AC-O5 dependency — 2026-07-15

- Fresh KJ validation at unchanged SHA `bd18165` reproduced a released-user
  failure twice: literal `Alt n` missed its one-second complete-tab deadline
  while the managed rail and manual `watch-tab` watcher were live.
- KJ's synchronous heartbeat delivery can align with the rail's deliberate
  1.2-second metadata refresh barrier. Faster renewal increases overlap;
  slower renewal can violate KJ's 2.5-second watcher-loss fail-close lease.
  Cadence tuning therefore cannot establish the action-latency invariant.
- Task 91 must provide the nonblocking metadata boundary before KJ validation
  resumes. Preserve KJ's exact original-rail identity, startup-only native
  inventory, lease fail-close, and explicit orphan-cleanup contract; do not
  reintroduce watcher-side polling, retries, or another controller.
- After task 91 lands, rerun KJ's native congestion proof and obtain a new
  full-range `code_completion` parent for KJ before its captain-live AC-I1.

#### Captain approved per-tab restart-empty contract — 2026-07-18

- The approved end value is one fail-closed in-memory registration set per
  manually started tab watcher. A watcher restart begins with zero session
  rows and may repopulate only after a fresh matching `SessionStart`.
- This supersedes AC-O4's shared full-snapshot/local-intersection mechanism
  and its requirement that restart rehydrate one row. Do not add a shared
  registry, persistent recovery controller, watcher polling, or retries.
- AC-O2 may use the shipped executable zero-forbidden-call, permission,
  source, and handler-matrix proof instead of adding the proposed panicking
  host seam, provided validation independently proves every handler path stays
  nonblocking.
- Acceptance remains anchored to the usable fixed-width journey: the rail is
  layout-owned at 28 columns, a delivered row focuses its exact registered
  pane while metadata enrichment is held in flight, native pane/tab actions
  meet their one-second deadlines, two rails project exact `1/1/0`, restart is
  empty, later fresh delivery recovers, and cleanup is complete.
- Unchanged boundaries: no synchronous metadata host calls, exact pane
  identity and lifecycle authority, bounded/cancelable/coalesced enrichment,
  malformed/stale fail-close behavior, and KJ's startup-only inventory, lease,
  and orphan-cleanup contract.

#### Captain accepted `FIXED` cleanup as follow-up — 2026-07-18

- AC-I1's first live checkpoint passed the fixed-width layout: one 28-column
  rail appeared at left with one selected terminal at right, and the captain
  confirmed the native permission prompt was visible.
- The same checkpoint exposed `FIXED` in the production rail header. The
  captain expected that word to be debug-only, but explicitly accepted it as
  a non-blocking narrow UX follow-up rather than rejecting task 91.
- Task 91 remains in validation with the fixed 28-column layout, inert former-
  toggle inputs, fullscreen behavior, exact-pane authority, nonblocking
  metadata path, and cleanup accepted at this checkpoint.
- The follow-up removes the production `FIXED` marker and pins the intended
  user-facing header bytes without changing layout or interaction behavior.
  It must land through its own task or a compatible narrow render-UX task.

#### Cycle 1 — 2026-07-19 — rail expands to half-width after pane creation

- The captain created additional ordinary panes in the correct task-91
  candidate tab during AC-I1. Native state then showed rail `plugin_33` at
  `91` of `181` columns, not 28; it was tiled, visible, and not fullscreen.
  The live dumped layout records `pane name="sidebar" size="50%"` beside four
  terminal panes.
- The captain clarified that the demo procedure had previously focused and
  expanded a pane through the CLI. The 91-column state is therefore real but
  not yet classified as a released normal-workflow defect; it may be a demo-
  harness evidence defect or a bounded abnormal trigger.
- Route to implementation with the current tab preserved as the failing
  specimen. Reproduce two fresh cases before editing product code: ordinary
  pane creation from a 28-column tab with no CLI focus/expand, and the prior
  CLI focus/expand sequence followed by pane creation. Only the ordinary-path
  reproduction blocks task 91 as an outcome defect; CLI-only failure is
  triaged as evidence/deferred scope with its exact trigger recorded.
- Preserve exact-pane session authority, the per-tab restart-empty contract,
  nonblocking metadata behavior, inert former-toggle inputs, native fullscreen,
  standing-root isolation, and the separate non-blocking header-label task.

#### Cycle 2 — 2026-07-20 — post-ready metadata delivery kills watcher

- In the fresh repaired live tab, the captain started the supplied wrapper and
  observed `watch-tab ready pid=47030`; rail `plugin_38` remained 28 columns
  and the empty projection correctly hid the `AGENTS` section.
- Before a usable session row appeared, PID `47030` exited. Its owned log
  records revision `2037` failing after five seconds without a recipient
  acknowledgment for `kind=metadata-snapshot`, targeted at session `WORK`, tab
  `4`, rail `38`, and the generation socket. The captain then observed no
  watcher process and no `AGENTS` row.
- The captain identified the actual AgentsView process still alive as PID
  `33838`, listening on `127.0.0.1:8080`. Direct API inspection then returned
  HTTP 500, `counting sessions: sql: database is closed`; the earlier
  disposable-data-root status had not described this live process. Classify
  that moment as alive-but-unhealthy, not absent. The same endpoint later
  recovered without a process replacement and returned HTTP 200 with 1,433
  sessions, matching the captain's successful server-backed session listing.
  Separate the transient source interval from the recorded plugin-pipe
  acknowledgment timeout before choosing a repair. Do not ask the captain for
  another blind retry.
- This blocks AC-I1 and the exact session-delivery value until a fresh normal
  journey keeps the watcher alive, positively acknowledges post-ready metadata,
  and renders one exact focusable row. Preserve the fixed-width repair,
  restart-empty contract, nonblocking action deadlines, cleanup, standing-root
  isolation, and the separate header-label follow-up.

## Stage Report: ideation

- DONE: Design a bounded cached/event-fed metadata architecture that removes every synchronous pane command, CWD, and scrollback call from the WASM hot path while preserving exact pane authority and degraded stale-state rendering.
  The proposal makes `PaneUpdate` plus KJ registration authoritative, uses atomic sequenced snapshots, removes `ReadPaneContents`, and specifies fresh, stale, unknown, absent, and ambiguous behavior.
- DONE: Spike the riskiest scheduling and cancellation path first, proving slow or non-shell pane enrichment cannot delay repeated pane/tab actions or create a later burst.
  The isolated Zellij 0.44.3 spike held six-second work through native observation; pane/tab actions completed in 19–24 ms, concurrency stayed at one, 100 revisions coalesced to the latest, and no late publication or state change appeared.
- DONE: Specify structured full-lifecycle pane observations and an in-flight-through-native-observation barrier, reusing task 44's exact refresh identity and adversarial wrong-pane/wrong-field variants.
  AC-O1 and the first test-plan step require complete lifecycle tuples, exact event/plugin/refresh/pane IDs, pre-send deadlines, an early-release negative, and a six-second post-cancel quiet window.

### Summary

Task 91 removes metadata acquisition from the WASM runtime instead of making
its blocking calls more elaborate. KJ supplies exact session identity,
`PaneUpdate` supplies exact live pane state, and the existing native subscriber
supplies bounded, cancelable, atomic snapshots; unavailable enrichment becomes
stale or unknown without guessing or delaying Zellij actions.

## Stage Report: implementation

- DONE: Reconciled current main's fixed-width managed rail with task 91 at
  code head `45719f4aaf545213b2a623871892ec87bde800ca`.
  Merge `f8ccc1b` contains main `999ba8a`: one layout-owned 28-column rail,
  inert `Alt /`, inert `FIXED` header, native fullscreen, no runtime layout
  mutation, and no `Reconfigure`. Conflict resolution retained task 91's
  exact-pane snapshots and removed main's superseded synchronous command/CWD
  polling, backoff, wedge drill, and `ReadPaneContents` permission.
- DONE: Corrected malformed snapshot degradation in `37bc0d1`. A malformed
  accepted-recipient update now retains the last good exact row, marks it
  visibly stale, and requires a later increasing healthy full snapshot to
  clear stale state.
- DONE: Corrected exact session click authority in `164c0ea`. A `PaneUpdate`
  may temporarily disarm future broadcast admission, but it no longer makes a
  cached session row inert while the exact registered pane remains live in the
  current manifest. Missing, moved, suppressed, or unselectable membership
  still clicks to nothing.
- DONE: Added the fixed-width usable congestion journey in `2643aee` and bound
  its latency to completed native observation in `45719f4`. The demo starts the
  manual watcher in the exact direct-entry terminal, renders
  `SMOKE_SECOND_ROW`, adds a real non-shell pane, holds native enrichment in
  flight, clicks the delivered session row, observes its exact registered pane
  focused before the one-second deadline, verifies the rail remains at
  `x=0,y=1,28x46`, then runs literal `Alt p` x3, `Alt n`, `Alt 1`, and `Alt 2`
  before the barrier is owner-released. It retains the six-second no-burst and
  owned-cleanup checks plus early-release, expired-barrier, and action-timeout
  negative controls.

### TDD evidence

- Malformed retained snapshot RED:
  `cargo test tests::agent_snapshot_requires_valid_payload_and_exact_recipient -- --exact --nocapture`
  failed at `src/main.rs:1973` because `snapshot_stale` remained false.
  The identical focused command passed after `37bc0d1`; the full Rust suite is
  80 passed, 0 failed.
- Manifest-refresh click RED:
  `cargo test tests::current_manifest_projects_cached_session_by_exact_lifecycle_state -- --exact --nocapture`
  failed with `left: None`, `right: FocusPane(4)`. The identical focused
  command passed after `164c0ea`; the full Rust suite and `cargo check --tests`
  passed at final head.
- Live reconciliation RED first launched `watch-tab` in a newly populated pane
  without the injected private route (`watch-tab missing explicit route
  context`). The harness now records the sole direct-entry terminal before
  population and focuses that exact ID before launch.
- Usable focus RED reached the owner-held metadata barrier but missed the
  one-second focus deadline because local click authority reused the temporarily
  unarmed pipe-recipient proof. The Rust fix above plus the canonical SGR row
  mapping made the same live journey green.

### Green verification

- Shared native target: `CARGO_TARGET_DIR=/Users/clkao/git/zaphod/target`,
  `RUSTC_WRAPPER=sccache`.
- `cargo test`: 80 passed; `cargo check --tests`: passed.
- `cd grout && go test ./... && go vet ./...`: passed.
- `env -u CARGO_TARGET_DIR ./tests/build-artifact-test.sh`: passed.
- `./tests/zellij-responsive-proof-test.sh`,
  `./tests/sidebar-scrollback-docs-test.sh`,
  `./tests/codex-session-hook-test.sh`, `./tests/zellij-new-tab-test.sh`, and
  `./tests/zellij-manual-permission-grant-test.sh`: passed.
- `env -u CARGO_TARGET_DIR ./tests/zellij-pane-metadata-congestion-test.sh`:
  passed the usable focus journey, literal action deadlines, three negative
  controls, quiet window, and cleanup.
- `env -u CARGO_TARGET_DIR ./tests/zellij-two-rail-recipient-smoke-test.sh`:
  passed exact same-CWD `1/1/0`, exact row focus, stale cache, restart-empty,
  bounded fetch, zero idle native polls, and standing-root isolation.
- `env -u CARGO_TARGET_DIR ./tests/zellij-watcher-lifecycle-smoke-test.sh`,
  `./tests/zellij-watcher-layout-stress-test.sh`, and
  `./tests/zellij-stress-evidence-test.sh`: passed outside/inside, serial,
  concurrent, injected failure/hang, and cleanup cases.
- Clean worktree; `git diff --check` passed.

### Usable demo evidence

The final retained positive run is under
`/tmp/zaphod-task91-review-fix.ltQjcE`. Its exact result was:

```text
PASS: delivered SMOKE_SECOND_ROW focused exact pane 1 in 81ms through the fixed 28-column rail while native metadata remained in flight
PASS: literal Alt p/Alt n/Alt 1/Alt 2 met native one-second deadlines during a native metadata barrier; the six-second post-close state was stable
```

An earlier retained run at `/tmp/zaphod-task91-usable-demo.SL88Bt` measured
23 ms and contains the visible `SMOKE_SECOND_ROW`, phase log, process ownership,
and cleanup result. The barrier phases are ordered
`responsive-metadata-barrier-entered` -> `responsive-session-focus-complete`
-> `responsive-metadata-barrier-released` -> `responsive-actions-complete`.

### Adversarial and asynchronous-boundary pass

- Identity/cardinality: exact pane ID is the only session join; same CWD,
  title, command, recency, foreign tab/token/rail, duplicate ID/pane ID, moved,
  suppressed, and missing panes do not create focus authority. Two rails render
  `1/1/0`.
- Protocol: empty, malformed, trailing JSON, Unicode, exact 1 MiB,
  maximum-plus-one, duplicate, old/repeated/out-of-order generation and
  sequence, stale, unhealthy, EOF, restart, and later healthy recovery are
  covered atomically.
- Fixed layout/input: `Alt /`, real `FIXED` click, and stale `toggle` pipe are
  layout-inert; native terminal and rail fullscreen restore non-focus identity,
  geometry, chrome, lifecycle, and layout fields.
- Native coordinator owns workers and publisher; the exact recipient is the
  original rail token/tab/pane tuple; `ready`/`accepted` are positive
  acknowledgments; fetch/publish/action deadlines are independent;
  supersession cancels generations; owned children are reaped; early release,
  expired ownership, late action observation, and post-cancel publication fail.

### Review evidence

- Exact-tip quick job `180` found one Medium evidence defect: the focus
  timestamp preceded native capture. `45719f4` records time after the successful
  capture and rejects a post-deadline observation. Replacement exact-tip quick
  job `182`, panel `quick`, passed with no findings.
- Authoritative full-range synthesis parent `188`, panel `code_completion`,
  reviewed
  `999ba8ab06af8c09a736aed98db21c0d70e341a0..45719f4aaf545213b2a623871892ec87bde800ca`.
  Required members `correctness` job `185`, `journey` job `186`, and `proof`
  job `187` each ran once and passed; no execution failures; parent verdict
  PASS with no findings.

### Required contract decision before implementation completion

The usable fixed-width journey is green, but this report does **not** mark task
91 implementation complete. The durable approved AC-O4 says both rails receive
one shared full bounded snapshot for local intersection and a sidecar restart
rehydrates exactly one row. The implemented/manual-watcher contract instead
keeps one registration in each watcher's memory and intentionally proves
`restart-empty` until a new SessionStart. README, the live two-rail harness, and
the demo all describe and enforce that safer restart-empty behavior, but the
entity's approved outcome was never updated. AC-O2 also asks for a panicking
forbidden-host seam through 100 manifest/timer cycles; final code has zero
forbidden calls and source/permission assertions plus handler matrices, but not
that named seam. The first officer must route a contract gate: either approve
the manual per-tab/restart-empty end value and update AC-O2/AC-O4 before
validation, or send implementation back to build the shared rehydrating
registry and exact requested host-seam proof. KJ's congestion rerun may use the
green demo mechanics and head evidence, but KJ must not be advanced from this
stage report alone.

## Stage Report: implementation (cycle 2)

- DONE: Align task 91's canonical AC-O2/AC-O4, approach, test plan, and documentation contract to the captain-approved per-tab restart-empty end value while preserving the recorded unchanged boundaries.
  Captain approval `137af5b` now governs one manual watcher and one in-memory registration per tab, restart-empty until fresh matching SessionStart, executable handler/source/permission proof, and no shared recovery registry.
- DONE: Map the existing fixed-width demo, exact 1/1/0 projection, restart-empty recovery, nonblocking handler coverage, and cleanup evidence to the revised acceptance criteria without inventing or rerunning unnecessary work.
  Code head `45719f4` and retained run `/tmp/zaphod-task91-review-fix.ltQjcE` plus the two-rail, Rust 80/80, Go, lifecycle, congestion-negative, quiet-window, and owned-cleanup results in the prior report cover the revised AC-O1 through AC-O6.
- DONE: Finish the implementation Stage Report with valid DONE/SKIPPED/FAILED accounting, commit the durable contract/report update, and leave task 91 ready for fresh independent validation.
  This documentation-only alignment changes no product code, preserves authoritative `code_completion` parent `188` at exact head `45719f4`, and leaves KJ's congestion rerun as the next-stage handoff.

### Summary

The captain resolved the prior AC-O2/AC-O4 gate in favor of the already-green,
fail-closed manual per-tab watcher contract. Canonical acceptance and proof now
match the fixed 28-column demo, restart-empty recovery, exact-pane authority,
and bounded nonblocking implementation, so task 91 is ready for fresh
independent validation without another build or verification run.

## Stage Report: validation

- DONE: Verify frozen code head 45719f4 and stored code_completion parent 188 cover the exact current merge-base range, include every required member once, and have an authoritative PASS verdict.
  Clean head `45719f4`, merge base `999ba8a`, quick `182`, and `code_completion` parent `188` were verified directly; correctness `185`, journey `186`, and proof `187` each occur once at exact range, `done/P`, retry zero.
- DONE: Independently reproduce every revised offline AC and run a throwaway-checkout refutation audit, including fixed 28-column usability under held metadata congestion, exact-pane authority, 1/1/0 projection, restart-empty then fresh recovery, fail-close behavior, deadlines, and cleanup.
  Rust 80/80, uncached Go plus vet, congestion, two-rail, and cleanup-stress packets passed; five local negative correctness mutations were rejected and the detached checkout was removed.
- DONE: Prepare the exact captain-live demo script plus Subspace gate artifact and decision record, clearly separating independently reproduced offline evidence from the interactive observation reserved for the captain.
  `gates/nonblocking-pane-metadata-architecture-validation.md`, the Subspace brief, and its decision log record offline PASS and an exact AC-I1 script; AC-I1 remains explicitly pending captain observation.
- SKIPPED: Run or claim the AC-I1 captain-live observation.
  The workflow makes the captain the validator for the real floating-TUI journey; no offline harness result was substituted for that observation.
- SKIPPED: Record a human Subspace Review v1 decision.
  Subspace 0.8.0-beta.5 opened the frozen brief at SHA-256 `a5c56086…f5ec7c` with `No decision`; no `person:reviewer` acted, so no Review v1 result was claimed and the pending presentation is recorded durably.

### Summary

All six revised offline criteria pass fresh independent validation at frozen
head `45719f4`, including the fixed-width usable journey, exact per-tab
restart-empty authority, deadlines, fail-close behavior, and complete owned
cleanup. The gate packet is ready for the captain's real Subspace/Codex AC-I1
walkthrough; product code and standing Zellij configuration were unchanged.

## Stage Report: validation (cycle 2)

- DONE: Run the exact frozen-head AC-I1 preflight at 45719f4 and stop with concrete evidence if any required build or congestion check is not green.
  Head `45719f4`, merge base `999ba8a`, Zellij 0.44.3, `build.sh`, and `zellij-pane-metadata-congestion-test.sh` all passed; the congestion wrapper again reported responsive native actions and quiet cleanup.
- DONE: Prepare the disposable live environment: services healthy, fresh managed tab launched, standing KDL hashes captured, and exact tab/permission/watcher commands recorded without changing standing configuration.
  Disposable AgentsView v0.37.5 is healthy; tab `4`, rail `plugin_33` at 28 columns, terminal `26`, entry record, exact commands, native JSON, layout, and identical before/after standing KDL hashes are retained.
- DONE: Stop at the first captain-only keypress or visual observation, provide one concise action cue, and retain native-state instrumentation without claiming AC-I1 until the captain reports the observation.
  The gate artifact asks the captain only to inspect `Task 91 captain live`, report the fixed rail/terminal view and permission-prompt presence, and not approve or start the watcher; AC-I1 remains pending.
- SKIPPED: Approve a permission prompt, start the manual watcher, send captain keypresses, or claim visual results.
  Those actions and observations belong to the captain; setup stopped at the prescribed boundary.

### Summary

The exact offline preflight is green and a disposable live AC-I1 environment
is ready at stable tab `4` with unchanged standing KDL bytes. Native setup
evidence and route commands are retained, while the first visual check and all
later keypresses remain explicitly captain-driven and unclaimed.

## Stage Report: validation (cycle 3)

- DONE: Record the captain's first AC-I1 visual observation without broadening it.
  The captain reported PASS for one fixed 28-column left rail and one selected terminal to its right; prompt visibility was not reported and is not inferred.
- DONE: Inspect only native instrumentation that does not substitute for the captain's visual report.
  Checkpoint JSON preserves tab `4`, rail `plugin_33` at 28x58, selected terminal `26`, intact chrome, healthy AgentsView, and unchanged standing KDL hashes.
- DONE: Provide the next single captain action while keeping AC-I1 pending.
  The only cue asks whether a permission prompt is visible and requests a `YES` or `NO` reply with no keypress, approval, or watcher start.
- SKIPPED: Infer prompt visibility from the rail's native selectable state.
  `is_selectable=false` is consistent with granted-state behavior but is not a human-visible prompt observation.

### Summary

The captain passed the first fixed-width visual checkpoint, and native state
remains consistent with the prepared environment. AC-I1 is still pending; the
next boundary is solely the captain's report of permission-prompt visibility.

## Stage Report: validation (cycle 4)

- DONE: Record the captain's permission-prompt observation exactly.
  The captain reported `YES`: a native permission prompt is visibly present; task 91 remains in validation and AC-I1 is not yet claimed.
- DONE: Record the captain's disposition of the visible `FIXED` label without treating it as a task-91 defect.
  The label is an accepted non-blocking follow-up owned by task `1zhcvrj8727eez45mdj6ec3j`, which may remove the debug-only text without behavior change.
- DONE: Preserve non-human instrumentation at the permission checkpoint.
  Native state, healthy AgentsView, and unchanged standing KDL hashes were rechecked without substituting them for the captain's visible prompt report.
- DONE: Provide exactly one next captain action with its expected visible result.
  The sole cue asks the captain to approve the prompt once, confirm it closes back to the unchanged fixed-width tab, report PASS/FAIL, and not start the watcher.

### Summary

The captain confirmed the native permission prompt and explicitly kept the
debug-only `FIXED` label outside task 91's blocking scope. AC-I1 continues from
the prompt-approval boundary; no later watcher or session behavior is claimed.

## Stage Report: validation (cycle 5)

- DONE: Record the captain's exact permission approval action without inferring its visual outcome.
  The captain confirmed one approval of the visible native prompt; prompt closure and layout preservation remain unclaimed pending explicit observation.
- DONE: Retain native-state and standing-file instrumentation after the action.
  Tab `4`, rail `plugin_33` at 28x58, selected terminal `26`, intact chrome, and unchanged standing KDL hashes are recorded without substituting for the human view.
- DONE: Detect and preserve the service boundary before watcher startup.
  The disposable AgentsView daemon is no longer listening; validation will restore it only after this visual checkpoint and before starting the watcher.
- DONE: Return exactly one next captain observation with its expected result.
  The sole cue asks for PASS only if the prompt is gone and the same fixed-width rail/terminal layout remains, otherwise FAIL; no further action is authorized.

### Summary

The captain's single permission approval is durably recorded, while its visual
result remains correctly pending. Native geometry and standing KDL bytes are
stable, and watcher startup is held until both the captain's view passes and
the disposable AgentsView service is restored.

## Stage Report: implementation (cycle 3)

- DONE: Reproduce the smallest ordinary pane-creation sequence that changes the correct task-91 rail from 28 columns to 50%, using the preserved live specimen as evidence, and classify narrow supported fix versus design reset before editing product code.
  Preserved tab `4` still showed `plugin_33` at 91/181 columns and `size="50%"`; fresh case A at `/tmp/zaphod-task91-width-A.DgTwOw` went 28 -> 91 on its first literal `Alt p` with no CLI focus/expand, so this is an AC-I1 normal-workflow outcome defect. Case B at `/tmp/zaphod-task91-width-B.TtJmdx` reconstructed CLI focus plus fullscreen expand/restore and then also went 28 -> 91; the original captain CLI commands were not durably recorded, so B is not claimed as exact. A temporary native fixed-swap probe at `/tmp/zaphod-task91-width-swap-probe.LVADB1` held 28 through three pane creations, classifying a narrow supported layout fix rather than a design reset.
- DONE: If Zellij 0.44.3 admits a narrow supported fix, add a red-first regression and preserve exact 28-column geometry through dynamic pane creation; otherwise make no workaround and record the concrete unsupported boundary for a captain decision.
  RED `ZAPHOD_SMOKE_PREBUILT_ARTIFACTS=1 ./tests/zellij-fixed-width-pane-creation-test.sh` failed at `/tmp/zaphod-task91-width-red.eDhkGB` with one fresh 160-column tab widened from 28 to 80 after exactly one new terminal. Commit `4949599` adds one passive `fixed-width` swap template plus literal-key geometry coverage; `b5a379f` requires exact two-terminal cardinality before the proof may pass. GREEN `/tmp/zaphod-task91-width-green.8nCINK` records `size=28`, valid KDL, owned cleanup, and unchanged standing roots; post-fix 181-column cases A and reconstructed B at `/tmp/zaphod-task91-width-A-fixed.Zqbzwo` and `/tmp/zaphod-task91-width-B-fixed.JsCUlh` remain 28 through three additions.
- DONE: Preserve all accepted task-91 metadata/session/fullscreen/isolation behavior, run only verification required by the resulting change, obtain current exact-head review evidence if code changes, and write a complete implementation feedback Stage Report.
  `zellij-new-tab-test.sh`, the focused fresh-pane test, manual permission proof, two-rail `1/1/0`/restart-empty proof, docs check, and full congestion smoke passed; `/tmp/zaphod-task91-width-full-green.PYuOKu` focused the exact session pane in 87 ms, kept all later literal pane/tab actions prompt, round-tripped native fullscreen, and cleaned every owned root. Rust and Go suites were not rerun because neither runtime changed. Quick job `227` found the missing post-wait terminal-cardinality assertion; `b5a379f` fixed it and exact-tip quick job `232` passed. Authoritative `code_completion` parent `239` reviewed `999ba8ab06af8c09a736aed98db21c0d70e341a0..b5a379f5ef32d7b92effb219631c286dd7d41185`; correctness `236`, journey `237`, and proof `238` each passed once, with parent verdict PASS and no findings.

### Summary

Ordinary pane creation—not the undocumented CLI prelude—reproduced the
fixed-width failure on the first literal key, making it a released outcome
defect. Zellij 0.44.3's native single-swap boundary admits a narrow fix with no
plugin runtime layout command; the corrected head preserves the 28-column
journey and all accepted task-91 behavior and is ready for fresh validation.

## Stage Report: validation (cycle 6)

- DONE: Verify replacement head b5a379f and code_completion parent 239 cover the exact current range, include required members 236/237/238 once, and pass without unresolved material findings.
  Clean head `b5a379f`, merge base `999ba8a`, parent `239`, and members correctness `236`, journey `237`, proof `238` were verified directly at exact range, `done/P`, retry zero, with no parent findings.
- DONE: Independently reproduce the normal supported path from a fresh 28-column tab: the first and three subsequent literal pane additions retain exact width 28, with a wrong/missing passive swap rejected, fullscreen/congestion/session behavior intact, and standing roots unchanged.
  Focused first-pane and full congestion journeys passed; missing swap widened to 80 and wrong swap rendered 29, both exiting 1; Rust 80/80, Go/vet, entry, permission, two-rail, fullscreen, timing, cleanup, and standing-hash proofs passed.
- DONE: Retire the preserved failing specimen after capturing it, prepare a fresh captain AC-I1 tab from b5a379f, and stop at the first human visual checkpoint with exact native geometry; do not claim the live observation.
  The 91-column `plugin_33` specimen and 50% dump are retained under `/tmp`, its exact panes were closed, and fresh tab `4` has `plugin_38` at 28x49 beside selected terminal `31`; AC-I1 remains pending.
- SKIPPED: Infer the replacement tab's visual result or continue to watcher/key actions.
  The gate stops at the captain's first look and asks only for the fixed-width view plus permission-prompt visibility.

### Summary

The narrow passive-swap repair passes exact-range review, independent positive
and negative width proofs, and every preserved task-91 contract. The old
failing specimen is retired, a fresh replacement tab is instrumented with
unchanged standing roots, and AC-I1 has restarted at its first captain-only
visual checkpoint.

## Stage Report: validation (cycle 7)

- DONE: Record both captain observations from the fresh replacement tab exactly.
  The captain reports the rail is visibly 28 columns wide and no permission prompt is visible; no permission approval is inferred or attempted.
- DONE: Preserve native instrumentation without substituting it for the human view.
  Tab `4`, `plugin_38` at 28x49, selected terminal `31`, intact chrome, healthy AgentsView, and unchanged standing KDL hashes remain recorded.
- DONE: Prepare the owner-held metadata barrier before watcher startup.
  Disposable root `.task91-live-b5.Rkweve` carries exact absolute enable/entered/release paths for the manually started tab watcher.
- DONE: Return exactly one next captain action with its expected visible result.
  The sole cue is one exact `watch-tab` command; expected output is ready PID/log, width 28, and empty AGENTS before SessionStart, followed by a stop.

### Summary

The replacement tab passes its first human fixed-width checkpoint without a
permission prompt. Native state and standing roots are stable, AgentsView is
healthy, and AC-I1 advances only to the manual watcher-start observation.

## Stage Report: validation (cycle 8)

- DONE: Record the captain's watcher failure output verbatim and retain its evidence.
  Human output was exactly `watch-tab daemon exited before readiness: EOF`; terminal transcript and watcher log `zaphod-watch-tab.2382221432.log` are preserved.
- DONE: Classify setup/evidence defect versus product defect from the retained route, binary, barrier, process, and service state.
  The long cue split into three commands, omitting barrier enable from the daemon; the log failed closed before socket/readiness, while AgentsView was also down. Candidate head/binary and tab geometry remained exact, so no product defect is established.
- DONE: Preserve cleanup and standing roots while restoring only the disposable dependency.
  No watcher/socket/barrier child remained; FO restored isolated AgentsView PID `75707`, its endpoint returns one session, and standing KDL hashes are unchanged.
- DONE: Provide exactly one safe retry action.
  Checked wrapper `.task91-live-b5.Rkweve/start-watcher.sh` removes multiline ambiguity; the sole cue runs it and expects ready PID/log, width 28, and empty AGENTS before stopping.

### Summary

The first replacement watcher attempt failed because the live-demo command was
split and its disposable source was down, not because watcher behavior was
exercised and rejected. Evidence is preserved, cleanup is complete, the source
is healthy again, and AC-I1 remains pending one safe wrapper-based retry.

## Stage Report: validation (cycle 9)

- DONE: Record the captain's successful wrapper readiness output exactly.
  The captain reported `watch-tab ready pid=47030 log=/Users/clkao/Library/Application Support/org.Zellij-Contributors.Zellij/zaphod-watch-tab.2525861830.log`; preceding display text was truncated to `tcher.sh`.
- DONE: Inspect the supporting watcher route and process without replacing human observation.
  PID `47030` owns the exact candidate binary, readiness log, generation socket, and live AgentsView connection; the log binds session `WORK`, tab `4`, terminal `31`, and rail `38`.
- DONE: Recheck native pane geometry, empty-projection support, and standing roots.
  Native state remains rail `plugin_38` at 28x49 beside focused terminal `31` at 153x49; a targeted rail dump is zero bytes and standing KDL hashes are unchanged.
- DONE: Return exactly one remaining captain observation.
  The sole cue asks for PASS only if the rail is still visibly 28 columns wide and AGENTS is empty, otherwise FAIL with the difference; no Codex or pane-key action is authorized.

### Summary

The short wrapper successfully started the exact watcher and its native route
is healthy. Native geometry and the blank rail dump support the expected state,
but AC-I1 remains pending the captain's post-readiness visual confirmation of
fixed width and empty AGENTS.

## Stage Report: validation (cycle 10)

- DONE: Record the captain's empty-state observation exactly.
  The captain reports no AGENTS heading is visible; no session row or other visual result is inferred.
- DONE: Interpret the observation against the frozen render contract.
  At `b5a379f`, `src/main.rs` wraps the AGENTS heading and rows in `if !sessions.is_empty()`, so zero delivered sessions intentionally hide the entire section.
- DONE: Preserve native watcher and geometry support without substituting it for the captain.
  Rail `plugin_38` remains exactly 28 columns beside focused terminal `31`; watcher PID `47030`, its generation socket, and AgentsView connection remain live.
- DONE: Return exactly one next captain action from the correct project CWD.
  Because terminal `31` currently reports CWD `/Users/clkao/git/agentsview`, the sole command changes to the frozen task worktree and starts `codex`; expected result is the TUI plus exactly one top-level Codex row, or a hook-trust prompt that must stop the sequence.

### Summary

The captain's missing AGENTS heading is the designed empty projection and
passes this checkpoint. AC-I1 now advances to starting one real top-level Codex
session in the watched terminal; no session-row result is claimed yet.

## Stage Report: implementation (cycle 4)

- DONE: Separate the transient AgentsView failure from the post-ready pipe
  failure and reproduce the released path against an independently owned
  source before editing product code.
  PID `33838` remained the actual listener on `127.0.0.1:8080`: its API
  returned HTTP 500 `sql: database is closed` during the rejected interval and
  later recovered to HTTP 200. The isolated-root `agentsview serve status`
  result described a different ownership root, so it never established that
  PID `33838` was absent. Independent AgentsView v0.38.1 PID `68294` now owns
  `127.0.0.1:18091`, serves HTTP 200 from the preserved isolated database, and
  exposes the exact registered session. This proves the source can be healthy
  while the recorded `revision=2037` failure remains a distinct recipient-
  acknowledgment defect.
- DONE: Add a red-first regression and the smallest authority-preserving fix
  for post-ready metadata delivery after ordinary pane changes.
  RED `cargo test harmless_manifest_refresh_keeps_exact_snapshot_recipient_live`
  failed at `src/main.rs:2030` because `sidebar.pipe(...)` returned false after
  a harmless same-tab `PaneUpdate`. Every `PaneUpdate` had incremented the
  manifest generation and cleared `agent_recipient`; normal focus or pane
  creation does not necessarily emit the later `TabUpdate` required to re-arm
  it, so the watcher could report ready from its initial acknowledgment and
  then lose every later snapshot acknowledgment. Commit `7bdb3d7` carries the
  previously proved stable-tab recipient forward only while the same plugin ID
  remains tiled at the same display position. Moved, floating, and missing
  variants still clear admission and were added as an adversarial matrix.
  No synchronous pane metadata call, poll, retry controller, layout mutation,
  or alternate identity source was added.
- DONE: Prove one stable, acknowledged, usable live journey and preserve every
  accepted fixed-width, session, timing, restart, cleanup, and isolation
  boundary.
  Fresh `WORK` tab `5` loads head `7bdb3d7` as rail `plugin_41`; independently
  owned watcher generation 2 is still live as PID `96455`, bound to terminal
  `33`, and renders exact AgentsView session
  `codex:019f7007-8fba-7503-8c44-5ebf9a7cc945` as one Codex row with the visible
  summary `You totally got this. Ta`. After readiness and row delivery, another
  ordinary pane was created and seven seconds elapsed beyond the former
  timeout: the watcher and socket remained live, the row remained rendered,
  and native inventory showed three selectable terminals beside one tiled rail
  at `x=0,y=1,28x49`.
  GREEN focused tests passed 2/2 and the Rust suite passed 82/82; `cargo check
  --tests`, Go tests/vet, the two-rail `1/1/0` and restart-empty smoke, the
  first-pane fixed-width smoke, and the full congestion journey all passed.
  The latter focused `SMOKE_SECOND_ROW` onto exact pane `1` in 81 ms while
  enrichment remained in flight, kept literal pane/tab actions within one
  second, and completed the six-second quiet cleanup. Exact-tip quick parent
  `293` passed. Authoritative `code_completion` parent `297` reviewed
  `999ba8ab06af8c09a736aed98db21c0d70e341a0..7bdb3d7a5a07b45245b37ee44d80920f673041b4`;
  correctness `294`, journey `295`, and proof `296` each ran once with
  `done/P`, and the synthesis verdict is PASS with no findings.

### Summary

The source had one transient alive-but-unhealthy interval, but it was not the
cause of the acknowledged post-ready pipe failure. Ordinary `PaneUpdate`
events incorrectly discarded a still-valid exact rail recipient. Head
`7bdb3d7` preserves that proof only across harmless same-rail refreshes and
continues to fail closed for movement, floating, disappearance, stale stream,
or foreign identity. The fresh 28-column live demo now holds one exact row and
survives later pane creation with its independently owned source and watcher
still running, so it is ready for renewed captain validation.

## Stage Report: validation (cycle 11)

- DONE: Verify frozen head and authoritative exact-range review integrity.
  Head `7bdb3d7`, merge base `999ba8a`, synthesis `297`, and members correctness `294`, journey `295`, proof `296` were verified directly on exact range `999ba8a..7bdb3d7`; all are `done/P`, retry zero, members occur once, and no material finding remains.
- DONE: Independently prove harmless post-ready refresh stays live while adjacent recipient states fail closed.
  Isolated Rust passed 82/82 plus check; exact two-rail and full congestion journeys passed live exact-row focus and post-ready ordinary pane creation. Moved/floating/missing/foreign matrices rejected delivery. Under-retention and over-retention-without-live-guards mutations each failed the targeted assertion with status 101.
- DONE: Preserve the accepted width, timing, restart, cleanup, and isolation boundaries.
  First and later literal pane additions stayed at width 28; all measured pane/tab actions completed under one second during held enrichment; restart-empty/fresh recovery, six-second quiet cleanup, permission, entry, docs, Go/vet, and retained failure-evidence suites passed.
- DONE: Prepare a validator-owned live source and unique-artifact tab without using implementer-owned evidence.
  AgentsView v0.38.1 PID `33168` at `127.0.0.1:18092` and proof tab `6` independently kept watcher PID `48779` plus its socket live seven seconds after post-ready PaneUpdate and exact delivery. A restarted watcher later failed after a validator post-ready rename on the previously used URL; its log is retained and tab `6` was retired rather than handed off. Frozen artifacts were copied byte-identically to a unique URL, and fresh tab `7` now has rail `plugin_47` at 28x49 beside selected terminal `38`; watcher startup correctly stops at the new URL's ungranted native permission boundary.
- SKIPPED: Claim the first fresh-tab visual result or any later AC-I1 action.
  Validation stops with exactly one captain look at width, selected-terminal cardinality, empty AGENTS projection, and permission-prompt visibility; no consent, Codex, Subspace, or pane keys are authorized yet.

### Summary

Recipient-repair head `7bdb3d7` passes exact review, independent positive and
fail-closed lifecycle proofs, adversarial refutation, and every preserved
offline contract. A validator-owned source and unique frozen-artifact tab are
stable at the first native permission checkpoint; AC-I1 remains pending the
human TUI journey and no watcher readiness is claimed.

## Stage Report: validation (cycle 12)

- DONE: Record the captain's unique-URL permission facts without broadening them.
  The captain reports the native prompt was visible in `Task 91 validator unique 7bdb3d7` and approved it exactly once; width, terminal cardinality, prompt closure, and empty AGENTS remain unclaimed.
- DONE: Inspect post-approval native state without substituting it for the captain's view.
  Stable tab `7` retains unique rail `plugin_47` at 28x58 beside sole focused terminal `38` at 210x58 with intact chrome, no validator watcher/socket, a zero-byte targeted rail dump, and unchanged standing KDL hashes.
- DONE: Preserve the service dependency boundary before watcher startup.
  Validator AgentsView PID `33168` has stopped and port `18092` is not listening; it is unnecessary for this visual-only checkpoint and must be restored before any later watcher action.
- DONE: Return exactly one next captain observation.
  The sole cue asks for the still-missing visible fixed 28-column rail, exactly one selected terminal, and absent AGENTS heading; watcher, Codex, and pane keys remain unauthorized.

### Summary

The captain has completed the one required native permission approval for the
unique frozen-artifact rail. Native geometry remains exact, but AC-I1 advances
only to the post-approval human visual checkpoint; no later state is claimed.

## Stage Report: validation (cycle 13)

- DONE: Record the captain's corrected current-tab visual result.
  Current `Task 91 validator unique 7bdb3d7` visibly passes fixed 28-column width, exactly one selected terminal, and no AGENTS heading; the captain's intermediate stale-row wording referred to a previous tab and is superseded for current tab `7`.
- DONE: Separate the prior-tab stale observation from current unique-URL state without inventing focus authority.
  Older canonical-URL rails remain in tabs `4` and `5` with no live watcher owner, but native state cannot recover the stale row's registered pane binding. Validation records the human prior-tab observation only, makes no last-good focusability claim, and does not treat it as current leakage.
- DONE: Restore the validator source and prepare one short health-checked watcher command.
  AgentsView v0.38.1 PID `90305` owns the same disposable root and serves HTTP 200 on port `18092`; executable `start-unique-watcher.sh` passes `sh -n`, preflights source health, and carries the exact tab-7/pane-38/unique-rail route.
- DONE: Return exactly one next captain action.
  The sole cue runs the short wrapper and expects ready PID/log while width stays 28 and AGENTS remains absent before SessionStart; Codex and pane keys remain unauthorized.

### Summary

The current unique frozen-artifact tab passes its complete post-permission
visual checkpoint. Prior-tab stale state is isolated and not overclaimed; the
validator source is healthy and AC-I1 advances only to exact watcher startup.

## Stage Report: validation (cycle 14)

- DONE: Record the captain's exact unique-watcher readiness output and post-start visual result.
  Human output was `watch-tab ready pid=18396 log=/Users/clkao/Library/Application Support/org.Zellij-Contributors.Zellij/zaphod-watch-tab.2004140966.log`; the captain separately confirms the current rail remains visibly 28 columns and AGENTS remains empty before SessionStart.
- DONE: Confirm native watcher, socket, source, and empty projection support beyond the old timeout.
  PID `18396` owns the byte-verified unique binary, exact tab-7 socket, and established port-18092 source connection; generation 2 binds WORK/tab `7`/terminal `38`/rail `47` and remained live for 81 seconds without post-ready error, beyond the former five-second acknowledgment failure.
- DONE: Preserve exact current native geometry without replacing human observation.
  Unique rail `plugin_47` remains 28x58 beside sole focused terminal `38` at 210x58 with intact chrome; the targeted rail dump is zero bytes.
- DONE: Return exactly one real-session action.
  The sole cue starts `codex` in the selected watched terminal and expects one top-level Codex row; a hook-trust prompt must be reported without proceeding, and no task prompt or pane keys are authorized yet.

### Summary

The unique watcher now passes both human and native readiness checkpoints and
survives the former failure window with an acknowledged empty projection.
AC-I1 advances to one real top-level Codex SessionStart and remains pending.

## Stage Report: validation (cycle 15)

- DONE: Record the captain's first-run hook-trust observation verbatim.
  The captain reported `TRRUST PROMPT`; the spelling is preserved, and the observation is classified only as the expected consent boundary for the checkout-local hook.
- DONE: Keep consent and session delivery explicitly unclaimed.
  The report does not prove trust, hook execution, SessionStart delivery, or a rendered row; AC-I1 remains pending.
- DONE: Identify the exact hook definition and required restart boundary.
  The project-local source is the frozen worktree's `.codex/hooks.json`, and its SessionStart command is `scripts/zaphod-codex-session-hook.sh`; after explicitly trusting only that definition through `/hooks`, the already-started process must exit and a new `codex` must start in watched terminal `38`.
- DONE: Return exactly one next captain action with a hard stop.
  The sole cue covers exact-hook trust plus the required restart, expects AGENTS with exactly one top-level Codex row, and stops before any task prompt or pane key.

### Summary

The unique-tab journey reached the expected Codex first-run trust boundary.
No consent or session delivery is inferred; AC-I1 advances only to explicit
trust of the exact checkout-local hook and a fresh Codex start in terminal
`38`.

## Stage Report: validation (cycle 16)

- DONE: Record the captain's pre-prompt no-row failure as observed.
  The captain trusted the exact hook, exited, restarted `codex` in terminal `38`, and saw no AGENTS section or row.
- DONE: Locate the first unavailable evidence boundary without rejecting the product.
  Native state has `pane_command=codex` in the exact unique CWD, while healthy validator AgentsView has zero sessions and no Codex session file for that CWD before any first prompt; the first unavailable boundary is therefore exact-ID enrichment.
- DONE: Recheck watcher route and post-ready health.
  PID `18396`, the exact tab-7 socket, and its port-18092 connection remain live; the generation-2 log still binds WORK/tab `7`/pane `38`/rail `47` and contains no post-ready delivery error. In-memory hook acceptance remains unclaimed.
- DONE: Correct the validation-script ordering and return one safe action.
  The original gate requires the `TASK91_CAPTAIN_REAL` prompt before row enrichment. The sole cue sends that first prompt, expects exactly one matching terminal-38 Codex row, and stops before pane keys or Subspace.

### Summary

The reported empty pre-prompt rail is real but was judged against a premature
checkpoint. It is a validation-ordering defect, not a product rejection.
AC-I1 remains pending the marker-prompt enrichment and all later live actions.

## Stage Report: validation (cycle 17)

- DONE: Record the captain's marker-prompt section observation without broadening it.
  The captain reported `I see AGENTS now`; this establishes only that the AGENTS section appeared, not row cardinality, identity, focusability, or pane binding.
- DONE: Confirm exact source identity and persisted marker support.
  Validator AgentsView and the local Codex session file agree on session `codex:019f7ee7-672a-7a72-9726-ecf1e0a4ce35`, exact unique CWD, agent `codex`, and the full `TASK91_CAPTAIN_REAL` first message.
- DONE: Recheck the live exact route without substituting native support for vision.
  Watcher PID `18396`, its tab-7 socket and AgentsView connections remain live with no post-ready log error; unique rail `plugin_47` remains 28x58 beside sole focused selectable terminal `38` running `codex` in the exact unique CWD.
- DONE: Return exactly one next captain observation before interaction.
  The sole cue asks for one top-level Codex row, no additional rows, and visible marker identity; click, pane keys, and Subspace remain unauthorized.

### Summary

The marker prompt made the AGENTS section appear and exact source support is
coherent. AC-I1 remains pending the human row-cardinality and marker-identity
check before any focus or timing action.
