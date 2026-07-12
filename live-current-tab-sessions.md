---
title: Live sessions arrive and lead back to work
status: implementation
source: captain direction 2026-07-11; Sprint 2 outcome shaping
sprint: s2-dependable-per-tab-attention-loop
group: walking-skeleton
sprint-readiness: ready
blocked-on:
blocked-reason:
score: 1.0
started: 2026-07-12T00:02:21Z
completed:
verdict:
worktree: .worktrees/spacedock-ensign-live-current-tab-sessions
issue:
pr:
mod-block:
id: bb3sedraaa53wa7wjp8xf0p7
---

## Problem

After Sprint 1 opens a safe managed Zaphod tab, an operator still has to run a
one-shot helper or hunt for the pane where an agent session is working. The
shipped rail already accepts `agent-event` session rows and can focus an
unambiguous CWD-bound pane, but current `grout` fetches one supplied session
and exits.

## Required outcome

**Trigger:** a real AgentsView session changes while one initialized managed
Zaphod tab is open. **Visible result:** its row appears in that tab's rail;
clicking the row returns focus to its unique originating terminal pane.
**Reproducible proof:** a source event, session-list response, recorded
`agent-event` pipe, and rail focus decision form one offline journey; after
Sprint 1 accepts its managed-tab smoke, one captain drill repeats that journey
against a real managed tab.

## Ideation boundary

Build one end-user attention loop only: the established `Alt Shift z` →
`scripts/zellij-new-tab.sh` entry creates a fresh managed tab, discovers that
tab's one resident rail natively, and starts one target-bound native sidecar.
That sidecar performs initial list, one `data_changed` subscription, common
attention-event emission through the existing Zellij/WASM bridge, and
exact-CWD focus inside that one tab. Sprint 1 supplies the successful
managed-tab creation surface; it is not a branch, lease, PTY, binding-core, or
rebase prerequisite. Reconnect hardening, row expiry, global-versus-tab gate
association, pooling, adoption, and multi-client refinements remain deferred.

## Proposed approach

### One end-user path

Keep `Alt Shift z` → `scripts/zellij-new-tab.sh` as the only user-facing
entry. The user never runs `grout subscribe`, `zaphod subscribe`, or a new
`zaphod workspace start` command. After the script has successfully created a
fresh managed tab, bb extends that same entry with one target-discovery and
sidecar-start step. The sidecar then turns real AgentsView changes into the
existing common attention event and delivers it through the Zellij/WASM rail
bridge. The rail remains the projection and action owner: it reuses
`apply_agent_event`, `rows_for_own_tab`, `bind_session`, and
`decide_rail_click`; one exact CWD match focuses its origin pane, while zero
or multiple matches stay unbound with no focus.

### Native artifact and post-create handoff

`build.sh` currently produces only the canonical WASM artifact. bb directly
adds one checkout-local native `zaphod` executable alongside that WASM build;
the build contract is that both artifacts exist before `new-tab` is attempted.
The native executable is not a second user entry or a controller: its private
sidecar invocation is `zaphod subscribe`. `grout` supplies internal source,
row-building, and pipe-adapter code behind that binary; it is not a documented
or installed public command.

The current script's successful `new-tab` call yields only raw `TAB_ID` and
canonical `WASM_URL`. Neither is a rail pane ID. After creation, the script
uses the same explicit Zellij binary/config/data/session arguments to make a
bounded native `action list-panes --json` wait. It must parse and verify
`TAB_ID` against native state, then accept exactly one resident record with
the verified tab ID, `is_plugin`, the exact `plugin_url == WASM_URL`, and the
resident non-floating rail shape. Its `id` becomes `RAIL_PANE_ID`. The current
smoke's non-suppressed, non-selectable 28-column rail is the concrete resident
fixture. Tab name, pane title, CWD, path prefix, a URL-only match, and the raw
new-tab output alone are never target discovery.

If that bounded wait sees no exact resident, more than one, malformed native
state, or an unparseable/mismatched tab ID, the entry reports
`sidecar-target-unready`, exits its post-create start path visibly, and does
not spawn a sidecar. It leaves the successfully created tab intact; it does
not retry forever, guess a target, or delete the tab. Once the exact record is
observed, the script starts exactly one private native process with the
complete target and the same Zellij profile, conceptually:

```text
<checkout>/target/zaphod subscribe --server <agentsview-url> \
  --zellij-bin <same-zellij> --zellij-config-dir <same-config-dir> \
  --zellij-config <same-config> --zellij-data-dir <same-data-dir> \
  --zellij-session <session> --tab-id <verified-tab-id> \
  --rail-pane-id <observed-pane-id> --rail-url <canonical-wasm-url>
```

This is bb's direct artifact, build, and invocation boundary. Successful
fresh-tab creation in the same entry is its only runtime condition; bb does
not create a separate prerequisite or revive bc, a binding core, a lease, or
a controller lane.

### Sidecar lifecycle ownership

The target-bound native `zaphod subscribe` sidecar owns one AgentsView
snapshot/SSE subscription, its local exact-target probe, and the short-lived
`zellij pipe` children it creates. It maps initial and `data_changed` list
results through the existing `BuildSessionRow`/`EmitRow` seam into the common
`agent-event` attention event. The current bridge is session-wide broadcast by
pipe name, never `--plugin`; the verified rail pane is a lifecycle guard, not
a claim of per-pane transport routing.

The sidecar rechecks its exact `{Zellij profile, session, verified tab ID,
rail pane/instance ID, canonical WASM URL}` target for its lifetime, including
before a pipe emission. Explicit `SIGINT`/`SIGTERM`, source EOF/failure, a
vanished session/tab/plugin, a URL mismatch, or a same-URL replacement at a
new pane/instance ID cancels its stream and owned pipe work. It reports a
terminal reason (`target-lost` for target loss) and exits itself. The script,
rail, and Sprint 1 entry do not supervise, retarget, restart, or clean it up.

Stopping deliberately does not heal. A malformed source record is reported
and skipped, but EOF, source failure, or target loss does not reconnect, retry
forever, auto-start AgentsView, follow a replacement, clear rows, or create a
replacement sidecar. The sidecar cannot stop AgentsView; delete, kill, close,
or reconfigure a Zellij session, tab, pane, or plugin; or alter the managed
tab configuration. Reconnect, row expiry, and pooling remain later work only
if the completed loop shows a measured need.

### Managed-tab integration contract

The established script supplies the fresh tab and its own explicit Zellij
profile arguments; bb owns only the native artifact plus the small post-create
observe-and-spawn extension. The extension consumes standard native
`list-panes` output, not a new managed-tab API. It consumes no
`ProfileLeaseV1`, second client, custom PTY, controller, or binding-core
surface. The accepted managed-tab smoke remains the integration and
captain-live gate, but it is not a separate implementation prerequisite or a
reason to rebase.

### No-rebase decision and current-main spike

No source-level dependency requires rebasing or consuming an unpublished
branch API. The selected entry is now on current `main`: its merge base with
`feature/zellij-new-tab-entry` is that branch tip (`fabfc73d`), so the entry
is already an ancestor rather than a pending source dependency. Its actual
post-`new-tab` contract is only raw `TAB_ID` plus canonical `WASM_URL`; bb's
native `list-panes` observation supplies the missing rail pane ID. Current
main's `build.sh` has no native `zaphod` artifact yet, which is precisely bb's
small direct build addition, not a rebase, bc, or binding-core prerequisite.
Current-main source has no `ProfileLease`/`PROFILE_LEASE` reference outside
documentation.

The current-main spike is green: `cd grout && GOPROXY=off go test -count=1
-run 'TestEmitEndToEnd|TestSessionRowFromFixture' ./...` passed in a fresh run.
It proves the existing source-fixture → `SessionRow` → bounded named-pipe
seam without a lease. The planned internal sidecar extends that seam behind
the native `zaphod` artifact; it does not import a Sprint 1 API. If
implementation later finds a concrete source API mismatch, it records that
mismatch and asks for a new decision rather than rebasing by assumption.

### Riskiest unproven mechanism and smallest spike

The first invalidating joint is the post-create handoff, not a lease: after a
fake native `new-tab` returns `TAB_ID=73` and a canonical WASM URL, a fake
`list-panes --json` sequence must move from not-ready to exactly one matching
resident plugin record. Only then may the fake native `zaphod subscribe`
process receive the observed pane ID and profile/session tuple. The hermetic
test fails if it uses a tab name, title, CWD, URL-only candidate, raw new-tab
output as a pane ID, a manual public grout command, or ambient target
discovery.

The next check is the narrow live arrival path: a loopback SSE server changes
its list fixture from empty to one session whose CWD equals the fixture's only
managed-tab terminal; the started sidecar must emit one existing-format
`agent-event` session row after `data_changed`. The test fails if the sidecar
needs a lease, branch API, a second tab, or a new user entry.

After the managed-tab smoke passes, the smallest captain-live drill uses the
same `Alt Shift z` entry, observes its native target handoff and one real
source session in the fresh terminal's CWD, then clicks the resulting row back
to that terminal. It does not exercise a gate, foreign tab, reconnect,
pooling, adoption, or second client.

## Acceptance criteria

### Offline (agent-reproducible)

**AC-O1** — The established fresh-tab entry starts one internal sidecar only
after exact native target discovery, and a real source arrival becomes a rail
row without a manual command. A fake `new-tab` yields `TAB_ID=73` and a
canonical WASM URL; a fake native `list-panes` wait becomes one resident plugin
record whose verified `tab_id`, URL, and pane `id` match. Only then does the
fake native `zaphod subscribe` receive the explicit Zellij profile/session/tab/
pane/URL tuple. Against its loopback SSE endpoint and fake AgentsView list
(empty → one fixture session after `data_changed`), it emits exactly one
existing-format `agent-event` session row. The expected ID, CWD, state, and
summary come from the source fixture, not from the adapter.

Verified by: a black-box entry test with fake build/new-tab/list-panes/native
sidecar recorders plus a Go loopback-SSE/fake-AgentsView/Zellij-payload test.
They assert start ordering, the exact target argv, emitted JSON, and no
title/CWD/URL-only target discovery.

**AC-O2** — The projected row leads back only to its originating managed-tab
pane. Feeding AC-O1's row to a rail fixture with one selectable terminal at
the same exact CWD yields `FocusPane(that pane)` on click. A missing CWD or
two matching terminal panes yields an unbound row and `ClickAction::None`.

Verified by: Rust tests around `apply_agent_event`, `rows_for_own_tab`,
`bind_session`, and `decide_rail_click`, using the CWD and pane ID fixture as
the external expected value. No global or cross-tab association is asserted.

**AC-O3** — bb directly owns the small native artifact/build/invocation
addition without a Sprint 1 branch rebase, separate prerequisite, or new user
entry. From current main, the build produces the existing canonical WASM and
one checkout-local native `zaphod` executable; its private `subscribe`
subcommand reuses internal `grout` seams rather than publishing `grout`.
Current main already contains the entry script, while
`feature/zellij-new-tab-entry` is its merge-base ancestor; no branch API,
ProfileLease, bc, binding-core, controller, or `zaphod workspace start`
surface is consumed.

Verified by: the recorded merge-base/path/lease audit, a build-artifact test,
and the fresh current-main Go spike (`TestEmitEndToEnd|TestSessionRowFromFixture`).
A missing native artifact, public grout invocation, branch/rebase dependency,
or lease/binding-core reference fails this boundary.

**AC-O4** — The post-create handoff and sidecar have bounded lifecycle
ownership. A zero, duplicate, malformed, non-resident, mismatched-tab, or
URL-only `list-panes` candidate reaches its bounded deadline as
`sidecar-target-unready` and starts no sidecar. Once the script has observed
the exact target, the sidecar owns its target probe, one SSE connection, and
pipe children. Session/tab/plugin loss, original pane/instance disappearance,
URL mismatch, or same-URL replacement at a new instance ID cancels that work,
emits `target-lost`, and makes no later pipe, restart, or retarget. SSE
EOF/source failure is likewise terminal. Neither path issues an AgentsView
stop or Zellij delete/kill/close/reconfigure command.

Verified by: shell handoff fakes and Go lifecycle tests with controllable
native target snapshots, fake SSE, and Zellij argv/payload/process recorders.
The fixtures assert failed-start visibility, exact initial target, terminal
reason, and cleanup only of sidecar-owned work.

### Captain-live (only after AC-O1 through AC-O4 and Sprint 1 smoke pass)

**AC-I1** — An operator sees one real current session in the initialized
managed tab and returns to its pane. In the managed-tab smoke environment,
`Alt Shift z` runs the established entry, which creates the tab, observes its
exact resident rail, and starts the internal sidecar. One real source session
under the terminal's CWD then appears in the resident rail within one source
event/list cycle. Clicking it focuses that same terminal, with no manual
grout/zaphod command, tab hunt, or second user entry.

Verified by: a captain drill retaining the source event/list observation,
before/after native pane snapshots, and visible rail click. It runs only after
the accepted Sprint 1 smoke; it is not substituted with 7h, a lease, or a
custom PTY result.

## Test plan

1. **Prove the entry handoff first.** Extend the fake fresh-tab test so native
   `new-tab` returns `TAB_ID=73`, the first list-panes snapshots are not ready,
   and a later snapshot has exactly one resident plugin record with matching
   verified tab ID and canonical WASM URL. Assert that the checkout-local
   native sidecar is spawned exactly once afterward with its observed pane ID
   and inherited Zellij profile; no title/CWD/URL-only fallback is accepted.
2. Cover every failed-start boundary: malformed raw tab ID, zero/duplicate
   candidates, wrong tab, wrong URL, floating/non-resident plugin, and bounded
   wait exhaustion. Each exits visibly as `sidecar-target-unready`, starts no
   sidecar, and leaves the freshly created tab alone.
3. Add pure Go sidecar tests for snapshot/event dispatch and a loopback
   AgentsView fixture (empty → one exact-CWD session). Its one SSE connection
   must emit the existing `agent-event` payload after `data_changed`; EOF/error
   is a visible terminal failure, not reconnect logic.
4. Add target-lifecycle fakes around `zaphod subscribe`: session/tab/plugin
   loss, original rail instance absent, URL/identity mismatch, and same-URL
   replacement at a new instance ID. Each cancels SSE/owned child work and
   reports `target-lost`, with no later pipe, retarget, restart, or external
   cleanup command.
5. Add Rust row-projection tests for the one managed-tab terminal, zero match,
   and duplicate CWD cases. Reuse the current `agent-event` protocol and click
   decider; do not add global/tab association state.
6. Run the native-artifact build test, focused Go suite under `GOPROXY=off`,
   relevant Rust tests, `cargo check --tests`, and `git diff --check` from
   current main. Re-run the ancestor/path/lease audit before integration.
7. After the managed-tab smoke accepts, run AC-I1's one-entry drill. A missing
   managed tab is a held integration gate, not a reason to rebase, revive 7h,
   or broaden the task.

## Documentation change

Update README agent-row guidance around the single `Alt Shift z` /
`scripts/zellij-new-tab.sh` journey. Document that it builds the checkout's
WASM plus native sidecar, waits for exact native rail identity after new-tab,
then starts the internal `zaphod subscribe` sidecar; `grout` is not a user
command. State that target loss, Ctrl-C, or source failure ends the sidecar
without daemon, session, tab, pane, or plugin cleanup. Source restart,
departure cleanup, replacement retargeting, multi-tab association, and review
behavior are not part of this first slice.

## Out of scope

- `7h`, `4d`, ProfileLeaseV1, bc, binding-core, custom PTYs, foreground
  process groups, lease publication, branch rebases, and consumption of stale
  Sprint 1 branch code;
- global-versus-tab gate association, gates or review controls, pooling,
  reconnect/backoff hardening, stale-row expiry, automatic source startup, or
  source-owned multi-session policy;
- automatic sidecar restart, retargeting to a replacement tab/session/rail,
  or sidecar authority to clean up AgentsView or Zellij-owned resources;
- a public `grout subscribe` workflow, `zaphod workspace start`, a second
  launcher, a hub/controller protocol, a second managed tab, cross-tab focus,
  or multi-client delivery refinements; and
- any standing Zellij configuration mutation outside Sprint 1's accepted
  managed-tab smoke.

## Rejected ProfileLeaseV1/watch-loop proposal (historical)

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

## Rejected acceptance criteria (historical)

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

## Rejected test plan (historical)

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

## Rejected documentation change (historical)

Update the README agent-row description and add a `grout watch` section. Say
that a watcher is explicitly bound to a disposable profile lease; it shows
only metadata-proven top-level sessions that match the rail's current tab;
an ambiguous match is visible but cannot focus; and a source outage makes a
row stale before the configured expiry. Document that the watcher neither
uses standing Zellij configuration nor starts or stops a shared source
daemon.

## Rejected out-of-scope boundary (historical)

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

## Stage Report: ideation (cycle 4)

- DONE: Replace the obsolete ProfileLeaseV1/7h watch-loop design with the smallest real session-arrives → visible row → focus-originating-pane outcome for one managed tab.
  AC-O1 now specifies one initial-list/SSE `data_changed` subscriber and recorded `agent-event` row; AC-O2 reuses the current exact-CWD click seam; AC-I1 holds the one-session managed-tab drill until Sprint 1 accepts. The earlier lease proposal is labeled rejected historical material.
- DONE: State and spike-test whether implementation can start from current main without a Sprint 1 branch rebase; treat Sprint 1 only as a managed-tab integration contract unless a concrete code API proves otherwise.
  AC-O3 records merge base `2e0810b`, no `grout/` branch diff or session-seam diff, no current-main lease reference, and the fresh current-main Go spike `TestEmitEndToEnd|TestSessionRowFromFixture` PASS. Implementation starts on main; Sprint 1 gates only integration/live proof.
- DONE: Revise the bb entity's ACs, test plan, out-of-scope boundary, and ideation report with exact evidence; do not write product code or revive 7h/4d/lease/custom-PTY scope.
  AC-O1 through AC-I1 cite the loopback arrival fixture, current rail pure seams, no-rebase audit, and held managed-tab drill. No product code, 7h/4d record, lease, custom PTY, branch rebase, gate association, pooling, reconnect hardening, adoption, or multi-client refinement changed.

### Summary

Cycle 4 resets bb to one operator journey: an AgentsView change reaches the
managed rail and returns the operator to one uniquely CWD-bound pane. Current
main already contains the row/payload seam, and the managed-tab branch adds no
subscriber API, so implementation need not rebase. Sprint 1 remains the
accepted managed-tab integration gate, not a lease dependency.

## Stage Report: ideation (cycle 5)

- DONE: Resolve the material lifecycle-ownership question for the one real AgentsView subscription.
  AC-O1 and AC-O4 now name bb's new `grout subscribe` runner as creator and lifecycle owner, with `{session, rail-instance, candidate-URL}` as its only target tuple and its own target probe, SSE connection, and pipe children.
- DONE: Make target loss and stop behavior checkable without widening the slice.
  Session/tab/plugin exit, URL mismatch, and same-URL replacement at a new instance ID now cancel the runner with `target-lost`; EOF/source failure is terminal; no reconnect, retarget, restart, or AgentsView/Zellij cleanup authority is introduced. The Go lifecycle tests execute the new runner in AC-O4 and test-plan step 3.
- DONE: Preserve the current-main, no-Sprint-1-rebase decision and the held integration gate.
  AC-O2 retains its exact-CWD/no-focus ambiguity fixture, AC-O3's recorded current-main evidence is unchanged, and AC-I1 remains the held Sprint 1 managed-tab drill. This revision changes only bb's design, ACs, test plan, documentation intent, out-of-scope boundary, and report. No product code, 7h/4d record, lease, custom PTY, branch rebase, or managed-tab implementation change was made.

### Summary

The revised walking skeleton has one deliberately owned subscription:
`grout subscribe` validates an observed resident rail, owns the resulting
stream, and ends it when that exact target or its source ends. It cannot
silently follow a replacement or clean up external resources. The bb ideation
fold is ready for captain re-presentation.

## Stage Report: ideation (cycle 6)

- DONE: Replace the tautological direct-grout start with the retained end-user entry and an internal native sidecar.
  AC-O1 and AC-O4 now make `Alt Shift z` → `scripts/zellij-new-tab.sh` the only user journey: after new-tab, a bounded native list-panes observation verifies `{tab ID, canonical WASM URL, resident rail pane ID}` before the script starts one private `zaphod subscribe`. `grout` is internal adapter/package code, never a public command.
- DONE: Make the artifact, handoff, and ownership boundary direct and testable for bb.
  AC-O3 assigns bb the checkout-local native `zaphod` build artifact plus the script's observe-and-spawn extension, with no rebase, separate prerequisite, bc, ProfileLease, binding-core, controller, or `zaphod workspace start`. The current entry supplies only raw `TAB_ID`/`WASM_URL`; the native snapshot, not a name or CWD, supplies the matching rail instance.
- DONE: Keep the established projection and held live proof while naming all terminal behavior.
  AC-O2 retains exact-CWD/no-focus projection; AC-O4 covers failed target discovery, target loss, replacement, and EOF with no external cleanup; AC-I1 is the one-entry managed-tab drill after smoke acceptance. This revision changes only bb's design, ACs, test plan, documentation intent, out-of-scope boundary, and report—no product code or other entity state.

### Summary

Cycle 6 supersedes cycle 5's direct `grout subscribe` proposal. The stable
entry creates and observes the tab; the private native sidecar owns the live
AgentsView-to-`agent-event` bridge and exits on loss of that exact target. The
bb ideation fold is ready for captain re-presentation.
