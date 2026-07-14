---
id: 91f2dxkn3v7fe1174ayj48j5
title: Remove synchronous pane metadata calls from the plugin hot path
status: implementation
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
hook and native registry identify an agent session's exact Zellij pane. CWD,
the foreground command, titles, prompt text, and temporal proximity no longer
authorize session binding. Task 91 must use that exact identity rather than
building a second pane-discovery mechanism.

## Required outcome

Make the plugin render from bounded cached/event-fed state only. Slow pane enrichment must run outside the WASM hot path with explicit deadlines, limited concurrency, cache/backoff, cancellation, and stale-state presentation. Exact pane identity remains authoritative; unavailable enrichment renders degraded or unbound rather than guessing. Add a congestion regression that exercises a slow/non-shell pane while repeated pane/tab actions remain prompt and do not burst later.

Implementation starts from a mainline that contains task `44` and KJ's
approved exact session-to-pane projection. If KJ has not merged, stop; do not
restore CWD binding or duplicate its registry to make this task independently
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
test below repeats the same barrier through the candidate sidecar, private
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

### 2. Feed one atomic, bounded cache over the existing private channel

Extend the KJ subscriber rather than add a watcher or second sidecar. Each
accepted full snapshot carries:

```text
stream_generation, snapshot_seq, observed_at, source_health,
sessions[{provider_session_id, pane_id, agent, state, summary, updated_at}]
```

The native registrar and AgentsView remain authoritative. Each tab-bound
subscriber reads the session-scoped registry and exact AgentsView records. It
does not call `list-panes` to enrich, filter, or bind rows. It may use the
entry-time target proof once; afterward, the private pipe acknowledgment
proves that the exact token-bound rail still exists. The plugin intersects the
snapshot with its current `PaneUpdate`, so every tab may receive the same
bounded session snapshot while only the rail containing an exact pane ID
renders or focuses that session. A pane move therefore changes projection on
the next `PaneUpdate` without a CWD lookup or a native inventory poll.

Validate a full snapshot atomically before replacing the cache. Reject the
whole record on a duplicate session ID, duplicate pane ID, noncanonical pane
ID, invalid state, malformed UTF-8, trailing JSON, or a payload beyond KJ's
existing 1 MiB registry/session envelope. Retain the last good cache and mark
its source unhealthy.

Persist a monotonic `stream_generation` for each recipient token under the
existing runtime lock. Within one generation, accept only increasing
`snapshot_seq`. A restarted sidecar reaps its predecessor, increments the
generation, and completes a private ready handshake before sending a full
snapshot. A new plugin ignores snapshots until that handshake arms the current
generation. It then rejects lower generations and old or repeated sequences,
so a canceled pipe cannot overwrite newer state if Zellij delivers it late.

### 3. Bound, cancel, and coalesce native enrichment

Add one latest-wins refresh coordinator to the existing Go sidecar. Extend the
pure KJ functions `deliverableRegistrations`, `registeredSessionsForTab`, and
`BuildRegisteredSessionRow`: split exact-session fetching from tab projection,
then let WASM perform the final manifest intersection. Do not create another
binding model.

- Registry changes, AgentsView `data_changed`, and the bounded recovery timer
  request a refresh generation. A newer request cancels the current generation
  and replaces the single pending generation.
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

These bounds apply per sidecar. They cannot congest Zellij's pane metadata
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
- Sidecar loss: the plugin's cache-age timer marks the last snapshot stale;
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

Verified by: the isolated tmux/Zellij harness, a sidecar debug barrier, and
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
the command, CWD, or scrollback host APIs. An active rail with 100 manifest
updates and 100 cache-age timer ticks issues zero such calls.

Verified by: a host seam that panics on every forbidden API, exercised through
all handler paths in `src/main.rs`, plus the compiled candidate's permission set. The test
asserts exact rows and focus decisions, not only call counts; the candidate no
longer requests `ReadPaneContents`.

**AC-O3 — scheduling, cancellation, and cache size stay bounded.** A burst of
100 refresh requests for one pane produces one latest completed generation,
never exceeds two enrichment workers or one publisher, and retains at most one
pending refresh and one pending snapshot. Closing the pane, canceling the
sidecar, or superseding the generation reaps every worker and pipe child. No
result or publish appears after its deadline or cancellation.

Verified by: a deterministic Go coordinator test in
`grout/metadata_scheduler_test.go` with blocked workers, explicit
start/cancel/release barriers, runtime worker/publisher gauges, and a
post-deadline quiet window. Adjacent tests cover two panes, out-of-order
completion, source timeout, pipe timeout, repeated cancellation, and a worker
that ignores cancellation until its deadline.

**AC-O4 — exact pane authority survives lifecycle and restart.** Two same-CWD
managed tabs receive the same bounded registered-session snapshot but each
renders exactly the one top-level session whose registered pane is in its
current manifest; a child and unrelated history render nowhere. Moving the
pane moves the row, closing or suppressing it removes the row, and plugin or
sidecar restart rehydrates exactly one row. CWD, command, title, time, and row
order cannot create a binding.

Verified by: KJ's two-tab disposable harness,
`tests/zellij-two-rail-recipient-smoke-test.sh`, extended with full snapshots,
native pane move/close/suppress state, plugin/sidecar restart, exact counts
`1/1/0`, click focus, and negative fixtures whose only matching attribute is
CWD, command, title, or recency.

**AC-O5 — stale and malformed data degrade without lying.** A source timeout,
sidecar kill, malformed snapshot, duplicate identity, over-limit snapshot, old
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
server, sidecar, workers, fixtures, and temporary roots. Standing config and
layout bytes remain unchanged.

Verified by: injected failures under the existing cleanup traps in
`tests/zellij-pane-metadata-congestion-test.sh`, bounded PID and session-absence
checks, pre/post standing-file digests, and retained phase
evidence. A missing dependency fails the required case instead of reporting a
skipped pass.

### Interactive (only after AC-O1 through AC-O6)

**AC-I1 — the captain can work normally beside a slow pane.** In a fresh
managed tab using main-built WASM, the captain opens real `subspace-tui` and a
real registered Codex session. The rail shows exact session state or a visible
stale marker. Three `Alt p` presses, `Alt n`, `Alt 1`, and `Alt 2` each take
effect in under one second; closing the exact sidebar and waiting six seconds
causes no delayed action burst.

Verified by: captain observation plus external native pane/tab snapshots,
sidecar gauges, exact refresh records, and standing-KDL hashes. The drill uses
a fresh managed tab and follows `docs/zellij-agentsview-live-demo.md`; it never
retrofits or hot-reloads an existing rail.

## Test plan

1. **Repeat the invalidating spike first through the production seam.** Port
   the passed native scheduler spike into a committed deterministic test. Add
   the candidate sidecar barrier and full snapshot, load the candidate WASM in
   the isolated tmux harness, and run AC-O1. Prove that releasing the barrier
   before native observation makes the test fail. If literal actions still
   miss one second, preserve the structured evidence and stop; do not add a
   longer timeout or another polling layer.
2. Rebase or merge only after KJ's exact registration work and task 44 are on
   the implementation base. Resolve the combined source so `registered_session_pane`
   replaces CWD binding and task 44's zero-scrollback rule remains intact.
3. Remove the WASM host calls and permissions. Extend `rows_for_own_tab`,
   `apply_agent_snapshot`, and the session projection as pure functions. Run
   the forbidden-host trap across every event and input path, then `cargo test`
   and `cargo check --tests`.
4. Add the Go latest-wins coordinator around KJ's
   `deliverableRegistrations`, `registeredSessionsForTab`, and
   `BuildRegisteredSessionRow`. Prove deadlines, two-worker/one-publisher
   limits, coalescing, cancellation, backoff, stream generation, atomic size
   limits, and child cleanup before changing the live subscriber.
5. Extend the KJ two-tab harness for same-snapshot local intersection,
   move/close/suppress, plugin and sidecar restart, source outage/recovery, and
   the `1/1/0` cardinality. Add adjacent wrong-pane, wrong-field, old-sequence,
   duplicate, EOF, Unicode, and maximum-plus-one variants.
6. Run the retained Rust, Go, artifact, new-tab, managed-tab, recipient,
   lifecycle, and layout suites. Run AC-O1's success and injected-failure
   cleanup modes from a fresh build. Verify `git diff --check` and standing KDL
   hashes.
7. Only after the offline packet passes, give the captain AC-I1. A live miss
   blocks validation and preserves evidence; it does not authorize in-session
   retrofit, manual watcher setup, or broader workspace-hub work.

## Documentation change

- Update README's Features, Permissions, Status, and Development sections:
  the rail uses manifest and exact registered-session snapshots, never
  periodic command/CWD/scrollback calls; stale and unknown states are visible;
  `ReadPaneContents` is no longer required.
- Update SPEC's historical status note and `docs/docking-approach.md` to retire
  all periodic WASM metadata calls, not only scrollback, while preserving the
  old prototype findings as history.
- Extend `docs/zellij-tmux-smoke-harness.md` with the external refresh barrier,
  full lifecycle tuples, early-release negative, worker/publisher bounds, and
  post-cancel quiet window.
- Update `docs/zellij-agentsview-live-demo.md` only if KJ has merged; keep exact
  registration as authority and add the visible stale/recovery step.

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
