---
title: Live sessions arrive and lead back to work
status: ideation
source: captain direction 2026-07-11; Sprint 2 outcome shaping
sprint: s2-dependable-per-tab-attention-loop
group: walking-skeleton
sprint-readiness: ready
blocked-on: 7h-validation-gate-before-implementation
blocked-reason: Ideation is captain-approved now. Implementation and the live drill require the passed disposable-profile gate.
score: 1.0
started: 2026-07-12T00:02:21Z
completed:
verdict: REJECTED
worktree:
issue:
pr:
mod-block:
id: bb3sedraaa53wa7wjp8xf0p7
---

## Problem

A normal Zellij tab does not continuously turn live top-level agent sessions into trustworthy, focusable current-tab attention.

## Required outcome

The current-tab rail continuously shows genuine top-level sessions for that tab, focuses an unambiguous bound pane on click, and removes departed sessions without manual one-shot commands or tab hunting.

## Ideation boundary

Design one profile-scoped subscriber path: initial, SSE data_changed, reconnect, and periodic list refresh; authoritative top-level-session filtering; current-tab binding; stale expiry; and bounded failure behavior. Use yb and hj as evidence, not as unchanged dispatches. Do not add gates, pane adoption, managed tabs, or review controls.

## Proposed approach

### Chosen boundary

Add one external, profile-scoped `grout watch --profile-lease <path>
--server <agentsview-url>` subscriber. The process reads the immutable
`ProfileLeaseV1` handoff and targets only that disposable Zellij profile. It
does not discover a session from ambient `HOME`, Zellij configuration, data
directory, or `ZELLIJ_SESSION_NAME`; it does not start, stop, or repair a
Zellij or AgentsView daemon.

The profile-target adapter consumes the lease's complete `attach` object
verbatim. Every `zellij pipe` invocation uses the lease's binary, exact
config/data/cache/home inputs, namespace, and native session ID through the
7h-defined argv/environment mapping. An invalid, missing, or vanished lease
or target session fails visibly before a pipe is sent. The watcher exits when
that target ends; it has no cleanup authority over the profile or a shared
AgentsView service.

This is deliberately smaller than both alternatives considered:

- Reusing yb's global watch path is rejected: it has no profile target and
  its `agent-` ID prefix misses real `codex:` children.
- Introducing a hub, managed tab, controller, or second client is rejected:
  none is needed to make the supported per-tab rail receive truthful rows.

### Subscriber lifecycle

After validating the lease, the watcher performs an immediate full list
refresh, then maintains one SSE connection to `/api/v1/events`. `data_changed`
is a trigger only: events carry scope, not session content, so every initial
load, event-triggered update, reconnect, and periodic tick re-lists the
source. A heartbeat resets the liveness deadline. The periodic refresh
(default 30 seconds) is required even with a healthy stream, because a
session leaving the source window has no guaranteed event.

Each successful refresh runs the source command with an explicit active
window and `--include-one-shot --include-children`, decodes the complete
source metadata, applies the top-level predicate below, and re-emits every
eligible row with a fresh RFC3339 `ts`. The watcher uses the existing
`BuildSessionRow`, `MapSessionState`, and timeout-bounded `EmitRow` seam; it
does not invent a second row protocol or batch format.

Only one refresh may run at a time. Events or ticks during it set one dirty
bit, yielding at most one follow-up refresh. EOF, a malformed SSE frame, or
heartbeat silence closes the stream and retries with bounded exponential
backoff (1 second through a 30-second cap); a successful reconnect always
does a full refresh. A list, source, or pipe failure neither clears rows nor
starts unbounded work: it leaves their timestamps unchanged, reports one
rate-limited diagnostic, and waits for the next bounded retry. The existing
five-second pipe timeout remains the per-row child-process limit.

### Authoritative top-level filter

Top-level is a source-metadata property, not an ID, agent name, CWD, or
AgentsView default-filter inference. The list deliberately includes children
so the subscriber can inspect `relationship_type` and `parent_session_id`.
It accepts a session only when the version-pinned root relationship shape and
an empty parent ID both prove that it is a root. A `subagent` relationship, a
nonempty parent, contradictory metadata, or an unknown relationship shape is
excluded. No fallback accepts `agent-`, UUID, or `codex:` ID shapes.

The first live-profile spike pins the exact root representation exposed by
the installed AgentsView version. If its root representation is an empty
relationship value, that empty value becomes an explicit, fixture-proven
root case; it is never treated as an implicit default. Any later unknown
source value fails closed and is surfaced as a source-compatibility error.
This replaces yb's prefix rule, whose validation record shows genuine Codex
subagents with `relationship_type: "subagent"`, a real parent ID, and a
`codex:` ID that the prefix rule retained.

### Current-tab projection and focus

The subscriber broadcasts the profile's eligible session rows once. Each
rail instance decides visibility from its current `PaneManifest`; the source
process never assigns a tab. Binding is globally unambiguous before it is
current-tab-local: collect every listed, selectable, non-plugin pane in the
profile's manifest, compare exact CWDs, and count matches without selecting a
tab. Extend `rows_for_own_tab`, `bind_session`, and `decide_rail_click` with
pure `global_session_candidates` and `project_session_for_own_tab` decisions:

1. Zero profile-wide CWD matches omits the session from every rail.
2. Exactly one profile-wide match renders a bound row only in that pane's own
   rail. Rails in every other tab omit it.
3. Two or more profile-wide matches render an unbound row in every rail that
   owns a matching pane. Each click is a no-op. Thus a source session whose
   CWD matches one pane in each of two tabs is unbound in both rails, not
   bound once per tab.

On click, `decide_rail_click` re-evaluates the same global candidate set and
may return `FocusPane` only when it still contains exactly one pane and that
pane belongs to the rail handling the click. A changed, missing, closed, or
foreign pane also makes the click a no-op. There is no `go_to_tab`,
`show_self`, session ID, title, path heuristic, active-tab value, raw tab ID,
or CWD-prefix fallback.

The manifest's tab membership only identifies where an already unique pane
may render; it never breaks a tie. Thus a stale CWD entry outside the latest
row set cannot bind, and focus can never cross tabs. The existing
`focus_terminal_pane` call remains the only action after the pure decision
has proved one profile-wide, current-rail pane.

### Freshness, expiry, and failure truth

`SessionRow.ts` is already emitted but the current `SessionEvent` discards
it. Extend the event model to retain a parsed observation time and add pure
`session_freshness` and `expire_sessions` decisions. A malformed timestamp
does not overwrite a prior good observation. A successful complete list
refresh is the only operation that advances a session's observation time;
absence from later successful snapshots and source failures both stop that
advance.

With a 30-second default tick, a row becomes visibly stale and non-actionable
after 90 seconds without a fresh observation, then expires after 120 seconds.
The thresholds are injected in tests. This avoids an immediate blank rail on
a transient outage while bounding a departed or unreachable session's ghost
lifetime. The timer removes only expired session rows; gate behavior is not
changed.

The implementation extends existing pure seams rather than replacing the
rail: Go's `decodeSession`, `BuildSessionRow`, `MapSessionState`, and
`EmitRow`; Rust's `apply_agent_event`, `rows_for_own_tab`, `bind_session`,
and `decide_rail_click`. It adds the small pure top-level, own-tab projection,
and freshness functions around them.

### Riskiest unproven mechanism and smallest live-profile spike

The riskiest unproven joint is not SSE parsing; yb already established that
`data_changed` needs a list refresh and that heartbeats/reconnects exist. It
is the complete lease-to-row path: profile-targeted piping plus an
authoritative root/child distinction on current AgentsView data.

Smallest invalidating spike, after 7h passes: start one attached disposable
profile, launch one watcher from its published lease, and create one genuine
top-level session with one real Codex or Claude child in the profile's only
terminal CWD. Capture a metadata-only
`agentsview session list --json --include-one-shot --include-children` set,
the watcher's recorded Zellij argv/environment, and the profile's live
`list-panes --json -a -g -t` output. The root must appear once in AGENTS, the
child must not appear despite the shared CWD, and clicking the root row must
focus that terminal in the leased session. A wrong profile target, an
unclassified root, a visible child, or any focus outside that tab invalidates
the design before a wider implementation.

The only implementation-only dependency is a **passed 7h validation gate**
that publishes its foreground-attached `ProfileLeaseV1`. It supplies a real,
isolated profile and exact target values; it grants no controller, pane
adoption, daemon, or cleanup authority. This task does not change 7h and has
no dependency on paused bc, qb, or 6v work.

## Acceptance criteria

### Offline (agent-reproducible)

**AC-O1** — One profile-targeted subscriber converges from initial load, SSE,
reconnect, and periodic refresh. Against a loopback SSE server with
recorded `data_changed` and heartbeat frames, a fake AgentsView list whose
snapshots change, and a fake Zellij binary, the watcher emits the expected
top-level rows immediately, re-emits a changed row after an event, performs a
full refresh after EOF/reconnect, and discovers an omitted row's departure on
the periodic refresh. The expected IDs, states, and timestamps come from the
server fixtures, not the watcher source.

Verified by: a hermetic Go test with an injectable clock and fake binaries;
the fake Zellij argv/environment must equal the supplied `ProfileLeaseV1`
target and must not contain an ambient profile value.

**AC-O2** — Root filtering is metadata-authoritative and fails closed. A
captured AgentsView list fixture contains one root, an `agent-` Claude child,
and a `codex:` child. Both children carry source relationship metadata and
share the root's CWD. The top-level predicate emits only the fixture-proven
root; an unknown, missing, or contradictory relationship value emits no row.
The test also proves the list request includes children for inspection rather
than relying on the source default to hide them.

Verified by: a Go list-decoding/filter test using the captured metadata-only
fixture and a fake AgentsView argv recorder. Its expected root/child table is
recorded outside the filtering function.

**AC-O3** — Current-tab scope, profile-wide ambiguity, and focus never
guess. A real single-line `list-panes --json -a -g -t` capture is adapted
into fixtures with a unique profile-wide CWD match, a foreign-only match, and
duplicate matches. The exact two-tab duplicate fixture has terminal pane 41
in rail A and terminal pane 84 in rail B, both with CWD
`/work/shared`, plus one source session with that same CWD. Its expected
result is an unbound row and `ClickAction::None` in rail A, and the same
unbound row and `ClickAction::None` in rail B; neither decision may produce
`FocusPane(41)` or `FocusPane(84)`. A unique profile-wide own-tab row remains
visible and decides `FocusPane`; a foreign-only row is absent. A stale CWD
map entry for a closed pane cannot alter any result.

Verified by: Rust tests for `global_session_candidates`,
`project_session_for_own_tab`, `bind_session`, and `decide_rail_click`,
including `two_tabs_same_cwd_never_binds_or_focuses`, followed by
`cargo test && cargo check --tests`. The two-tab fixture is recorded in
Zellij's real single-line `list-panes` JSON shape; no layout dump is parsed
or used for this task.

**AC-O4** — Failure is bounded and stale rows have a finite, truthful life.
A burst of `data_changed` during a blocked refresh causes no concurrent
refreshes and exactly one coalesced follow-up. EOF, silent SSE, list errors,
and a wedged pipe stay within the reconnect and five-second pipe budgets.
Rows remain unchanged on the first failed refresh, become stale and
non-actionable at 90 seconds, and are absent at 120 seconds; a fresh valid
timestamp restores them before expiry.

Verified by: Go single-flight/reconnect/wedge tests plus Rust injected-clock
freshness tests. The expected times are test inputs, and fake process PIDs
prove no child survives the timeout.

### Captain-live (only after AC-O1 through AC-O4 and 7h pass)

**AC-I1** — A real current-tab interruption appears and leads back to its pane
without tab hunting. In a fresh passed-7h profile, the metadata baseline
and the leased profile's `list-panes` output agree that exactly one
top-level, current-tab session is eligible. It appears after the initial or
SSE-driven refresh within the source's 10-second coalescing floor plus one
configured refresh interval; its real child does not appear. Clicking the
row focuses its bound pane and leaves every foreign tab unchanged. When the
source stops reporting that root, the row becomes stale and then disappears
within the configured 120-second bound without a manual grout command.

Verified by: the smallest live-profile spike above, including saved
metadata-only list output, profile-targeted argv, before/after pane snapshots,
and a captain observation of the row click.

## Test plan

1. **Run the smallest live-profile spike first, but only after 7h passes.**
   It invalidates an unsafe lease target or unsupported root/child metadata
   shape before implementation broadens the watcher. Record the real
   metadata-only root/child values and one-line profile pane snapshot; do not
   substitute an authored layout dump or an ID-prefix assertion.
2. Add the captured root/Claude-child/Codex-child list fixture and write the
   pure top-level predicate red first. Pin the explicit root value for the
   installed AgentsView version, reject unknown values, and verify the
   `--include-children` argv.
3. Add the lease-target adapter and fake-Zellij argv/environment test before
   any live pipe. It must reject missing/invalid lease data and prove no
   ambient Zellij values are read.
4. Add the watch loop tests: initial snapshot, `data_changed`, heartbeat,
   EOF/reconnect/full refresh, periodic disappearance, and single-flight
   coalescing with loopback SSE and fake AgentsView/Zellij binaries.
5. Extend the Rust event model with parsed timestamps, then test stale,
   expired, and renewed rows with an injected clock. Add current-tab scope,
   profile-wide ambiguity, closed-pane, and revalidated-click cases around
   the existing binding functions. Freeze the exact two-tab same-CWD fixture:
   rails A/B own terminal panes 41/84, both map to `/work/shared`, and the
   source session has that CWD. `two_tabs_same_cwd_never_binds_or_focuses`
   must return unbound/`ClickAction::None` for both rails and never any
   `FocusPane`, without a session-ID, title, path, active-tab, or raw-tab-ID
   tie-breaker.
6. Run `cd grout && GOPROXY=off go test -count=1 ./... && go vet ./...`,
   then `cargo test && cargo check --tests`. Only after those checks and 7h
   pass, run AC-I1's disposable-profile drill. A missing profile or source
   prerequisite is a visible failed drill, never a skipped green result.

## Documentation change

Update the README agent-row description and add a `grout watch` section. Say
that a watcher is explicitly bound to a disposable profile lease; it shows
only metadata-proven top-level sessions that match the rail's current tab;
an ambiguous match is visible but cannot focus; and a source outage makes a
row stale before the configured expiry. Document that the watcher neither
uses standing Zellij configuration nor starts or stops a shared source
daemon.

## Out of scope

- Gate discovery, review rows, provider actions, or inline verdicts.
- Cross-tab binding, tab switching, pane adoption, managed tabs, a hub,
  native launcher work, tmux, or a second profile client.
- Any change to 7h's ownership, foreground client, lease publisher, or
  cleanup; this task only consumes its passed handoff.
- ID-prefix, title, CWD-prefix, or default-list filtering as a substitute for
  source relationship metadata.
- Starting, stopping, authenticating, or otherwise owning AgentsView; multiple
  profile multiplexing; row batching; and changing the gate-row lifecycle.

## Stage Report: ideation

- DONE: Design one profile-scoped session-subscriber path covering initial load, SSE/reconnect/periodic refresh, and bounded failure behavior.
  The chosen lease-targeted watch loop has one initial/list path, scoped SSE triggers, bounded reconnect, single-flight refresh, and timeout-backed pipe behavior.
- DONE: Specify authoritative top-level filtering, current-tab binding, ambiguity handling, focus, and stale expiry with independently checkable evidence.
  Source relationship metadata replaces ID prefixes; fixture-based Go/Rust checks cover top-level classification, own-tab-only focus, ambiguity, and 90/120-second freshness boundaries.
- DONE: Name the smallest live-profile spike and the implementation-only dependency on 7h.
  The first post-7h profile drill validates lease targeting plus a real root/child source pair; no paused foundation lane is a prerequisite.

### Summary

The design keeps the existing per-tab rail and builds one profile-targeted
subscriber around it. It reuses yb's SSE/list evidence and hj's timestamp
idea while correcting their gaps: relationship metadata, not ID syntax,
defines top-level sessions, and the rail now owns bounded stale expiry. The
first real drill waits for 7h's foreground profile handoff and rejects any
wrong target, child leak, or non-local focus before wider implementation.

## Stage Report: ideation (cycle 2)

- DONE: Retain the approved subscriber design and add explicit Stage Report evidence mappings for every acceptance criterion, especially AC-O2 through AC-O4.
  AC-O1 → `Subscriber lifecycle` and test-plan steps 3–4: loopback SSE, fake AgentsView snapshots, a fake profile-targeted Zellij binary, and yb's recorded `data_changed`/heartbeat/list evidence (`grout-sse-daemon.md:38-66,97-125`).
  AC-O2 → `Authoritative top-level filter` and test-plan step 2: captured root/Claude-child/Codex-child metadata fixture plus fake list argv; the Codex false-negative and source `relationship_type`/`parent_session_id` oracle are recorded in `gates/grout-sse-daemon-validation.md:351-418`.
  AC-O3 → `Current-tab projection and focus` and test-plan step 5: a real one-line `list-panes --json -a -g -t` capture drives the own-tab/foreign/ambiguous cases; existing `rows_for_own_tab`, `bind_session`, and `decide_rail_click` are the cited pure seams in `src/main.rs:272-290,1349-1370,2003-2029`.
  AC-O4 → `Freshness, expiry, and failure truth` and test-plan steps 4–5: injected-clock 90/120-second cases, single-flight/reconnect/wedge fakes, yb's fresh-`ts`/stop-refresh seam (`grout-sse-daemon.md:112-125`), and hj's expiry seed (`rail-row-lifecycle.md:11-20`).
  AC-I1 → `Riskiest unproven mechanism and smallest live-profile spike` and test-plan steps 1 and 6: the post-7h lease-to-row drill captures source metadata, target argv, and before/after pane state; `foreground-attached-client-profile.md:40-57,112-125` supplies the held profile handoff. This is a planned live proof, not a claimed live result.
- DONE: Re-run the ideation AC scan and leave no unevidenced acceptance criterion.
  `spacedock status --read live-current-tab-sessions --stage ideation --ac-scan --json` is the gate-facing verifier; this cycle cites AC-O1, AC-O2, AC-O3, AC-O4, and AC-I1 inside checklist evidence rather than only in summary prose.
- DONE: Keep this as evidence repair only: no implementation, no scope growth, and no live profile work before the held 7h gate passes.
  This append changes only report evidence. The approved body, ACs, boundaries, 7h dependency, and deferred work remain intact; the AC-I1 drill stays explicitly held.

### Summary

Cycle 2 makes the ideation evidence auditable without changing the design.
Each AC now points to a concrete fixture, pure seam, source record, test-plan
step, or deliberately held live-profile proof. No code, profile, 7h, or 4d
state changed.

## Stage Report: ideation (cycle 3)

- DONE: Revise the design, AC-O3, and test plan so a source CWD shared by panes in two tabs cannot focus either pane.
  AC-O3 now requires profile-wide candidate counting before current-tab rendering: a shared CWD renders unbound in both matching rails, and `decide_rail_click` returns `ClickAction::None` unless one profile-wide candidate remains.
- DONE: Add the exact two-tab same-CWD fixture and test evidence without adding a heuristic tie-breaker.
  AC-O3 and test-plan step 5 name `two_tabs_same_cwd_never_binds_or_focuses`: panes 41 and 84 in rails A/B both map to `/work/shared`; the same-CWD source session is unbound with no `FocusPane` in either rail. The cited pure seams are `global_session_candidates`, `project_session_for_own_tab`, `bind_session`, and `decide_rail_click`; tab membership never selects a candidate.
- DONE: Keep the revision inside this session task and preserve the held live-profile boundary.
  This cycle changes only the session design, AC-O3, test plan, and report. It adds no code, profile run, 7h/4d mutation, session-ID/title/path/active-tab/raw-tab-ID tie-breaker, or other-task change.
  AC-O1 retains its `Subscriber lifecycle` loopback-SSE/fake-profile-target proof; AC-O2 retains its captured root/Claude-child/Codex-child metadata fixture; AC-O4 retains its injected-clock and bounded-process proof; and AC-I1 remains the deliberately held post-7h lease-to-row drill. Their cycle-2 citations and verification sources are unchanged.

### Summary

Cycle 3 makes CWD ambiguity profile-wide. A rail may focus only a pane that
is unique across every selectable terminal pane in the profile and belongs to
that rail; same-CWD panes in two tabs remain visible but inert in both rails.
The exact pure-fixture proof is specified for later implementation, while the
held 7h live-profile drill remains untouched.
