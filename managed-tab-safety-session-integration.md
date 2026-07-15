---
title: Integrate managed-tab safety with tab-bound session delivery
status: validation
group: walking-skeleton
sprint: s1-managed-tab-safety
sprint-readiness: ready
score: 1.0
source: captain direction 2026-07-13; V3/BB merge conflict
id: kjhq0t2h6drse6b32cqybggv
started: 2026-07-13T06:54:16Z
worktree: .worktrees/spacedock-ensign-managed-tab-safety-session-integration
---

## Historical cycle-1 contract (superseded)

The following contract is retained only as review history. The canonical
cycle-2 identity contract begins at the next `## Problem` heading and replaces
these CWD-bound acceptance criteria.

### Problem

The approved managed-only `Alt /` hardening branch and the merged tab-bound
session subscriber branch diverged from `6f130ee`. At this ideation snapshot,
`main` is `2809908` (the BB merge) and V3 is `b3b003a`; an ordinary no-ff
merge tree produces one conflict, `README.md`, and auto-merges all product
files. The unreadable resolution risk is not a source-level API mismatch: it
is losing either the V3 ownership proof or BB's direct-entry/session-lifecycle
contract while resolving prose.

The integration must also prove one complete operator journey. A fresh tab
built from selected `main` must carry V3's exact managed proof, start BB's one
stable-tab subscriber, show and focus a session only in that tab, and retain
V3's managed-only `Alt /` boundary. Existing isolated checks prove pieces of
that journey, but no current packet names the combined result.

### Required outcome

An operator can use a selected-checkout, main-built fresh managed tab whose
exact `zaphod_managed_tab "v1"` plus canonical WASM URL authorizes its `Alt /`
route. The direct entry starts exactly one BB subscriber bound to that tab's
native stable ID; a session source update appears only in that rail and its
row focuses the one bound terminal. The managed rail toggles, while an active
same-WASM/sidebar-shaped tab without the proof remains inert. README describes
all of those limits together. This repair never activates, installs, rewrites,
or otherwise changes standing Zellij config or layout.

### Proposed approach

### Exact merge shape and smallest branch

The implementation starts in a dedicated integration worktree from current
`main`, not from either feature worktree. Re-run the merge audit before any
write: its expected base is `6f130ee`, with `main=2809908` and
`spacedock-ensign/managed-tab-toggle-authorization=b3b003a`. The recorded
`git merge-tree --write-tree --messages` result is
`3a1eac4e09890f849009b8b742f3b9ef60d79aa3`: only `README.md` has unmerged
stages. The generated tree already contains both `ManagedRailProof` /
`active_tab_for_toggle_authorization` and BB's `recipient-tab-id` receiver
guard, post-create `--tab-id`/`--rail-url` sidecar handoff, and V3 marker
validation. If a moved `main` expands the conflict beyond README or removes
either contract, stop and return the audit rather than resolving by guesswork.

Create one no-ff integration merge of V3 into that worktree. Do not rebase BB,
V3, or main; do not make a second controller, lease, subscription entry, or
managed-tab registry. The only production resolution is the existing merged
code plus a coherent README. Test-only code may add one combined disposable
tmux smoke; it must use temporary config/data/socket/home roots and clean its
own source fixture and tmux server.

### One operator journey

The direct selected-checkout command remains the only full entry:
`scripts/zellij-new-tab.sh --session <name> --agentsview-url <fixture-url>`.
It builds the checkout-local WASM plus private `target/zaphod`, renders and
validates all three V3 rail declarations, creates one tab, verifies exactly
one resident rail in the returned stable tab ID, then starts one private
`zaphod subscribe` with that ID and exact rail URL. `Alt Shift z` stays a
tab-only shortcut; it opens its static candidate layout but never starts a
helper pane or subscriber. Global normal-mode `Alt n` is plain `NewTab`, pane
mode `Alt n` is `NewPane`, and `default.kdl` contains no Zaphod plugin; none
of those paths has the verified rail identity or may start BB subscribe.

The combined smoke creates a disposable source whose initial list is empty
and whose one `data_changed` refresh supplies a session with the managed
terminal's exact physical CWD and a distinctive fixture marker. It proves
the private subscriber's `agent-event` reaches only the verified target rail;
a same-CWD, same-WASM, tiled bystander lacking V3's two declarations never
renders or binds the marker. The test then proves a real session-row action
focuses the target terminal, a literal `Alt /` changes only the marked target
rail, and the same literal key leaves the active lookalike's native layout,
inventory, focus, and screen unchanged.

The riskiest remaining joint is real rail-row focus in the tmux-hosted client,
not a new subscriber protocol. First run a minimal disposable click probe:
deliver one already-targeted fixture row to a known marked rail, send the
actual terminal mouse sequence to that row, and inspect native pane focus. If
that does not reproduce Zellij click delivery, record the refutation and ask
for direction; do not replace it with a hidden focus side channel. Once green,
keep the same interaction inside the combined source-to-row smoke.

### Documentation resolution

Resolve the sole README hunk by retaining both paragraphs, in journey order:

1. V3 declares the exact marker plus observed URL requirement and says visual
   shape, CWD, title, geometry, URL substring, and same-WASM lookalikes do not
   enable `Alt /`.
2. BB documents the direct command's exact resident wait, one private
   subscriber, stable-tab recipient routing, bounded terminal lifecycle, and
   the fact that `Alt Shift z` does not start it.
3. The real-key packet lists the managed-toggle/lookalike smoke, the
   two-rail recipient regression, and the new combined journey smoke.

Replace main's stale sentence that V3 is merely pending with the merged proof;
do not delete the subscriber lifecycle text or the existing harness document.
State explicitly that this sidecar emits sessions only. Pending-gate discovery,
pooling, tab association for gates, provider review launch, and post-resolution
refresh remain S9/QT Sprint 2 work, not a side effect of this merge.

### Acceptance criteria (superseded)

### Offline (agent-reproducible)

**AC-O1 — The integration contains both contracts without a guessed merge.**
The integration head descends from current main and one no-ff V3 merge; its
recomputed merge audit has no conflict outside README. It contains V3's three
layout marker/URL declarations and strict route-offer/receipt proof, plus BB's
post-create native target wait, exact stable-tab sidecar argv, and receiver
guard.

Verified by: `git merge-base`, `git merge-tree`, and a path-scoped inspection
of the integration merge against the recorded base. The native V3 layout
validator and BB entry fake provide behavior evidence: a malformed/missing
marker or wrong URL prevents the direct entry, and a missing/duplicate/wrong
stable-tab resident prevents sidecar start.

**AC-O2 — One selected-checkout managed tab delivers and acts on one session.**
In a disposable tmux-hosted Zellij profile, the direct command creates one
marked, exact-URL rail in the returned native stable tab and starts one private
subscriber with that ID. A fixture `data_changed` event for the tab's one
terminal CWD renders its externally supplied marker only in that rail; a real
row action leaves that terminal focused. A same-CWD, tiled, same-WASM
lookalike that lacks V3 declarations receives no visible row or binding.

Verified by: the new combined tmux smoke's native `list-panes`/`list-tabs`,
screen captures, subscriber log/fixture protocol, and before/after focus
projection. Fixture stable IDs, CWD, and marker originate outside Zaphod's
layout/test assertions; the test compares target and lookalike independently.

**AC-O3 — Alt-/ remains owned by the marked tab in the complete journey.**
After the target row is visible, literal `Alt /` changes the marked rail's
known 28-to-1 geometry without replacing its pane/session identity. With the
unmarked lookalike active, the same literal bytes leave its tab-scoped native
layout, inventory, focus, loaded-plugin projection, and settled screen
byte-identical.

Verified by: the combined tmux smoke and the retained
`tests/zellij-tmux-smoke-test.sh`; neither test may use a custom PTY, profile
script, lease, controller, synthetic plugin launch, or standing Zellij root.

**AC-O4 — The user-facing boundary is coherent and gates stay deferred.**
README preserves BB's private-sidecar lifecycle and direct entry while stating
V3's explicit ownership proof and the three disposable smoke commands. It
does not claim that `Alt Shift z` subscribes, that a global binding selects a
worktree artifact, that either `Alt n` path has Zaphod ownership, or that this
session-only sidecar pools/reviews gates.

Verified by: a documentation review against the merged README, current
`docs/zellij-tmux-smoke-harness.md`, and the declared S9/QT boundaries.

### Captain-live (only after AC-O1 through AC-O4)

**AC-I1 — The captain can observe the exact combined journey without touching
standing Zellij state.**
Using the integration checkout and the same disposable tmux/profile roots,
the captain sees the fixture session row in the direct-entry tab, activates it
to return to the managed terminal, sees the target toggle, and confirms the
lookalike does nothing. This drill neither uses WORK nor writes the captain's
normal config/layout; it does not test a gate or provider review.

Verified by: the captured disposable profile, native target/tab IDs,
target/lookalike before/after state, and the source fixture log.

### Test plan

1. Recompute the no-ff merge shape before editing. If the merge no longer has
   exactly the recorded README conflict, stop and report its paths; do not
   force, rebase, or auto-resolve unrelated changes.
2. In the integration worktree, run the existing black-box
   `tests/zellij-new-tab-test.sh` first. After V3's layout helper is merged,
   this is the smallest joint invalidator: its installed layout must satisfy
   marker+exact-URL validation before BB's fake native resident can start the
   private sidecar with stable tab ID 73 and the selected profile tuple.
3. Before building the complete source fixture, prove one real row click in a
   disposable tmux client: targeted fixture pipe → rendered session row →
   terminal mouse sequence → native focused-pane projection. A failure is a
   design refutation, not permission to add a focus helper or custom terminal
   machinery.
4. Add one `tests/zellij-managed-tab-session-smoke-test.sh` journey test. It
   starts a temporary local AgentsView-compatible list/SSE fixture, invokes
   the direct script against isolated roots, and cleans the fixture, Zellij
   session, tmux server, socket/config/data/home root, and temporary logs.
   It proves AC-O2 and AC-O3 in one target/lookalike run.
5. Retain and run focused regressions: `cargo test -q`, `cargo check --tests`,
   `go test ./...` from `grout`, `tests/build-artifact-test.sh`,
   `tests/zellij-new-tab-test.sh`, `tests/zellij-tmux-smoke-test.sh`, and
   `tests/zellij-two-rail-recipient-smoke-test.sh`. Run `git diff --check`.
   These are all isolated checks; never invoke the parked worktree-profile
   script or a custom PTY harness.
6. Only after the packet is green, perform AC-I1 with the disposable profile.
   A source fixture, target rail, or click failure returns to this task's
   feedback path; it does not expand into S9/QT or alter standing config.

### Documentation change

Update only README's fresh-managed-tab section and real-key command block to
resolve the conflict as described above. Preserve the BB direct-entry and
stable-tab subscriber instructions, add the V3 ownership proof as a completed
constraint, and list the combined disposable smoke beside the two retained
smokes. Add one clear deferral sentence: gates are not emitted or pooled here;
their lifecycle remains S9/QT. Do not rewrite historical documents or turn the
evergreen architecture's later hub into a claim about this slice.

### Out of scope

- Changing persistent Zellij configuration or layouts, including a WORK drill
  that activates them; the repair uses disposable roots only.
- Replacing the AWK transformer, reworking the direct CLI entry path, adding a
  controller, registry, lease, binding core, helper pane, public grout command,
  custom PTY, or profile harness.
- Rewriting BB/V3 source behavior beyond their ordinary no-ff merge, except
  for the test-only combined journey fixture and README conflict resolution.
- `7h`, `4d`, `fq`, multi-client delivery, sidecar restart/retarget/pooling,
  or any automatic tab lifecycle management.
- Gate discovery, global or tab-bound gate pooling, provider review launch,
  provider resolution, and post-resolution refresh. Those remain S9/QT Sprint
  2 scope and this subscriber continues to emit sessions only.

### Stage Report: ideation

- DONE: Prove the exact V3/BB merge shape and preserve both delivered contracts.
  At `main=2809908`, V3=`b3b003a`, and merge base `6f130ee`, merge tree `3a1eac4` has only a README conflict; its generated source contains both V3 managed proof and BB stable-tab delivery.
- DONE: Specify the smallest integration branch and verification packet.
  One no-ff V3 merge in a main-based worktree resolves README only; the first joint entry test, a real row-click probe, one combined disposable tmux journey smoke, and retained Rust/Go/entry/tmux regressions form the packet.
- DONE: Keep standing Zellij config and layout outside this repair.
  The plan confines activation and source fixture work to disposable config/data/socket/home/tmux roots, keeps direct `zellij-new-tab.sh` as the only subscriber entry, and excludes Alt Shift z/Alt n, profile harnesses, and custom PTYs.

### Summary

The task now names one complete BB+V3 operator journey rather than a merge by
files: exact managed provenance, one stable-tab subscriber, target-only
session/focus, and managed-only toggle/lookalike inertness. README must retain
both delivered contracts and explicitly defer gate pooling and review behavior
to S9/QT; no standing configuration or new lifecycle mechanism is authorized.

## Feedback Cycles

### Cycle 1 — 2026-07-14 — captain rejected ideation

- The current design proves the recipient tab for delivery but does not prove
  which terminal pane originated an AgentsView session. Checkout CWD is a
  project hint, not session authority; live use admitted historical sessions
  and subagents, and the reviewed `yb` design independently demonstrated that
  the same session can appear in two tabs sharing one CWD.
- Reframe the walking skeleton around an explicit AgentsView-session-to-live-
  terminal-pane identity bridge. Unregistered, stale, ambiguous, child, and
  foreign-tab sessions must fail closed and render no row. Preserve the stable
  tab/recipient-token delivery proof; do not replace one inferred identity
  with another.
- Spike the riskiest mechanism before revising the design: from a real agent
  harness started inside a managed terminal, obtain its authoritative
  AgentsView session ID and `ZELLIJ_PANE_ID`, register the pair, and prove that
  two managed tabs with the same checkout each render exactly their own one
  top-level session while spawned subagents render nowhere. Record lifecycle
  behavior for pane move/close, agent completion/restart, and plugin/sidecar
  restart. If the harness cannot expose an authoritative session ID at startup,
  stop and return the failed probe rather than falling back to CWD, timing,
  title, prompt text, or newest-session inference.
- Revise the acceptance criteria and test plan around exact identity,
  cardinality, negative evidence, cleanup, and rehydration. The spike result
  must choose the smallest supported registration carrier and name its owner;
  implementation remains out of scope for this ideation rework.

### Cycle 2 — 2026-07-14 — captain rejected canonical structure

- Preserve the passed real-harness spike and the cycle-2 identity mechanism;
  the design direction is accepted and must not be reopened.
- Replace the superseded canonical Problem / Proposed approach / Acceptance
  criteria / Test plan / Out of scope sections with the cycle-2 contract
  instead of leaving the revision under a parallel heading.
- Preserve this feedback history, but make `spacedock status --read
  managed-tab-safety-session-integration --ac-scan` discover AC-O1 through
  AC-O6 and AC-I1 as the task's authoritative acceptance criteria. Remove or
  relocate stale cycle-1 ACs so they cannot drive a later dispatch or gate.

### Cycle 3 — 2026-07-14 — captain chose a manual tab-local watcher

- Replace the persistent shared registry design with the smallest walking
  skeleton: one manually launched `zaphod watch-tab` daemon in the one agent
  terminal for a managed tab. The daemon inherits exact Zellij session and
  pane identity, resolves its tab and original rail pane, and owns the
  AgentsView subscription, in-memory registration, exact focus, and delivery.
- The trusted Codex `SessionStart` hook supplies the authoritative agent
  session ID through a private Unix socket derived from that same Zellij
  session and terminal pane. Missing daemon, socket, pane, tab, or original
  rail fails closed and renders no row.
- Daemon, pane, tab, or rail closure removes authority immediately. Daemon
  restart may require restarting or re-registering the agent. The walking
  skeleton supports one watched agent terminal per managed tab; agents in
  later panes explicitly launch their own watcher.
- Remove persistent registry, destructive pruning, registry-directory
  propagation, same-name Zellij incarnation recovery, automatic watcher
  launch, multi-pane discovery, and restart rehydration from KJ. Those belong
  to the separately filed automation/recovery follow-up.
- Preserve reusable exact AgentsView-ID projection, stable-recipient delivery,
  pane focus, and fail-closed tests from frozen head `2fa8e8424d196465cd00bd091932a65d4ef01107`.
  Ideation must specify which current changes survive and which registry
  machinery is deleted; it must not edit product code.
- This materially changes the operator journey and authority lifecycle, so it
  resets KJ's review convergence budget only after the canonical contract and
  acceptance criteria are rewritten and approved at the ideation gate.

### Cycle 4 — 2026-07-14 — captain approved lease-owned silent-loss cleanup

- Native testing showed that a recurring watcher `list-panes` call can queue
  behind plugin refresh and delay ordinary Zellij actions even when the call
  is cancelled after 150 ms. Idle polling therefore contradicts AC-O5.
- Remove idle native pane polling. The plugin's current exact pane manifest,
  recipient proof, watcher generation, and 2.5-second lease own prompt row and
  focus fail-close after silent terminal, tab, rail, watcher, or heartbeat
  loss.
- Keep bounded native revalidation at startup, before acknowledged full
  snapshot delivery on hook/data changes, and during cleanup. A daemon whose
  pane disappears silently may remain until a later lifecycle check or
  explicit cleanup; prompt PID/socket teardown moves to the asynchronous pane
  observation or watcher-automation follow-up.

### Cycle 5 — 2026-07-15 — captain-live watcher startup failed

- Classification: **narrow fix — outcome defect** against AC-O1 and AC-O2.
  Both watcher launches in the shipped two-tab journey exited before readiness;
  their private logs report `watch socket path exceeds 103 bytes`.
- Exact failing boundary: `zellij-new-tab.sh` supplies the default root
  `/tmp/zaphod-watch-tab-v1-501`, while `watchSocketPath()` appends `watch-`, a
  64-hex digest, and `.sock`; the resulting path exceeds Darwin's 103-byte Unix
  socket limit. Unit and native harness roots were shorter and missed the
  shipped default.
- Preserve the approved manual-watcher design. Shorten the deterministic socket
  child name while retaining exact `{zellij_session, pane_id}` derivation, and
  add a regression that passes the actual shipped default root through the
  canonical socket-path validator on Darwin's limit.
- Re-run focused Go tests, the two-rail/lifecycle/congestion packets, exact-head
  Roborev review, and the same captain-live startup. Do not add a registry,
  controller, lifecycle supervisor, or alternate proof harness.

### Cycle 6 — 2026-07-15 — captain approved final narrow stale-socket repair

- Classification: **narrow fix — outcome defect** against AC-O6 and AC-I1.
  The captain's normal first-use retry left a dead watch socket after the
  readiness parent killed its child; the next watcher refused the stale path
  and required manual removal before the journey could continue.
- Exact failing boundary: restart admission in `listenWatchSocket()` treats
  every existing socket as live. The approved socket contract already permits
  removing an endpoint only after proving it stale, while requiring a live
  watcher to remain exclusive and undisplaced.
- Preserve the manual watcher and in-memory authority design. Implement only
  safe stale-socket discrimination/removal, prove that a live watcher can
  never be evicted, and retain fail-closed behavior for ambiguous ownership.
  Do not add a registry, supervisor, recovery controller, or durable record.
- This is the captain-authorized final narrow feedback cycle. Re-run the
  focused socket/lifecycle tests, the exact two-watcher regression, and one
  focused Roborev review before returning to the fresh two-tab live drill.

### Cycle 7 — 2026-07-15 — captain-live readiness cardinality failed

- Classification: **narrow fix — outcome defect** against AC-O1 and AC-I1.
  In the ordinary attached captain session, the exact rail answered the
  readiness probe immediately with `readyready`; the watcher required exactly
  `ready`, retried until its outer deadline, and exited before readiness.
- Exact failing boundary: Zellij 0.44.3 retains a plugin runtime per client and
  broadcasts CLI pipes to retained runtimes after attach/detach, concatenating
  their identical outputs. `list-clients` reports only connected clients, so
  one visible native rail may legitimately yield repeated acknowledgments.
- Accept one or more complete `ready` atoms only. Reject empty, partial, or
  foreign bytes; retain initial exact-one native rail resolution; and
  strengthen the immediate pre-delivery authority probe to reject any second
  matching native rail so repeated runtime acknowledgments cannot conceal a
  duplicate visible recipient.
- Add focused single/repeated/malformed response tests, duplicated-runtime
  readiness coverage, and duplicate-native-rail rejection. Preserve the
  manual watcher, in-memory authority, and all cycle-6 live/stale socket rules;
  do not add a registry, supervisor, or alternate proof path.

### Cycle 8 — 2026-07-15 — captain chose startup-only native authority

- Classification: **design reset** after cycle 7 exposed both an outcome
  defect and an evidence defect in recurring native Zellij calls. The watcher
  successfully projected one exact Codex row, then later native CLI traffic
  returned timeouts and malformed ANSI stdout while the session remained
  active. More timeout, retry, or serialization work is rejected.
- Keep the smallest walking skeleton: resolve and prove the exact terminal,
  stable tab, and original rail once during `watch-tab` startup. After ready,
  perform no native `list-panes` inventory or cleanup probe. AgentsView
  refreshes deliver through the existing token-bound acknowledged pipe only.
- The plugin's current `PaneUpdate`, recipient proof, watcher generation, and
  2.5-second lease own row and focus fail-close. A missing or moved terminal
  removes its row through the manifest; rail or watcher loss expires the
  lease. A silently orphaned daemon may remain until manual cleanup or the
  existing watcher-automation follow-up.
- Do not add retries, a CLI broker, shared serialization, registry, supervisor,
  durable recovery, or task-91 metadata architecture. Amend the canonical
  lifecycle ACs and tests to prove zero post-ready native inventory calls,
  exact projection, lease expiry, and explicit/manual process cleanup before
  implementation resumes.

## Problem

The frozen implementation at `2fa8e8424d196465cd00bd091932a65d4ef01107`
proves the important join—trusted `SessionStart` ID to exact live pane—but its
shared persistent registry creates more authority and recovery machinery than
the walking skeleton needs. It must prune incomplete native snapshots, carry a
registry root into every pane, distinguish same-named Zellij incarnations, and
rehydrate after process restart. Three review rounds found those boundaries
unsafe or underspecified.

KJ now proves a smaller value: one operator explicitly starts one watcher from
the terminal that will run the agent. Before reporting ready, that watcher
inherits `ZELLIJ_SESSION_NAME` and `ZELLIJ_PANE_ID` and uses one native
inventory to resolve the exact terminal, stable tab, and original rail. This
tuple is startup evidence only. After ready, the watcher performs no native
pane inventory or cleanup probe; it accepts one top-level agent identity over
a private pane-derived socket and retains the mapping only in memory. CWD,
title, prompt, timestamps, child labels, ID prefixes, and newest-session order
never admit or focus a row.

Post-ready authority is deliberately split at an existing boundary. The
watcher supplies exact AgentsView snapshots and heartbeat under the injected
recipient token and one random generation. The plugin's current `PaneUpdate`
manifest, current recipient proof, generation, and 2.5-second lease decide
whether a row exists or can focus. A missing or moved terminal is removed by
the manifest; a missing rail has no rendering surface; watcher or delivery
loss expires the lease. The daemon is not a post-ready Zellij lifecycle
supervisor and may remain orphaned until explicit manual cleanup.

The end state remains non-durable. A missing watcher delivery, terminal, rail,
exact AgentsView record, current recipient proof, or live lease produces no
usable row. A missing socket prevents a new registration but does not revoke an
already accepted in-memory identity by itself. Watcher restart starts empty and
may require agent restart or another trusted `SessionStart`. Automatic launch,
orphan cleanup, and recovery remain filed as
`tab-local-agent-watcher-automation.md` (task `6s0s2704zrms3med9n04mm4y`).

## Reused mechanism and smallest invalidating spike

The accepted field mechanism remains terminal identity inherited from the PTY
plus agent identity supplied by the lifecycle hook. Herdr, Superset, and cmux
all use that join; KJ retains it without their durable stores or inference
fallbacks. Frozen-head exact lookup, stable-recipient delivery, and pane focus
are already green and remain implementation inputs rather than new inventions.

The startup authority shape was spiked first on 2026-07-14 with Zellij 0.44.3,
tmux 3.6a, the candidate WASM, two same-CWD tabs, and disposable
config/data/socket/HOME roots. No standing Zellij file changed.

1. A hook attempt in the selected terminal before the watcher existed failed
   closed. The watcher then inherited session `kw12617`, pane `0`, used one
   lightweight `list-panes --json --all --state --tab` snapshot to resolve
   stable tab `0`, and selected original candidate rail `1`; same-WASM rail
   `2` in the other tab was not authority.
2. Both watcher and hook independently derived
   `watch-<sha256(session NUL pane)>.sock` beneath one mode-`0700` disposable
   runtime root. The socket was mode `0600`. One complete versioned envelope
   carrying the inherited session/pane and a valid `SessionStart` ID was
   accepted; the mapping existed only in watcher memory.
3. Later work at `af7860a` proved safe stale-socket admission, and
   `e1ddbb1` proved that retained per-client plugin runtimes may return repeated
   complete readiness and snapshot acknowledgments. A real retained-client
   run then projected exactly one bound Codex row before recurring native CLI
   probes timed out or returned malformed ANSI while the session remained
   active. That failure invalidates post-ready native revalidation, not the
   startup tuple, token-bound delivery, manifest binding, or lease.

The riskiest remaining combination is therefore explicit: after the one
startup inventory has returned ready, can current plugin manifest and lease
mechanisms keep exact projection fail-closed while the watcher makes zero
further native inventory calls? Implementation must exercise that path first
with an instrumented native-command boundary across registration, AgentsView
`data_changed`, heartbeat, terminal close/move, rail loss, and cleanup. Any
post-ready `list-panes` call or stale actionable row fails the design. No retry,
broker, registry, supervisor, or task-91 mechanism is an allowed repair.

## Proposed approach

### Exact operator journey

1. `scripts/zellij-new-tab.sh --session NAME` builds and creates one fresh
   V3-managed tab with one original rail and one initial terminal. It no longer
   starts a subscriber. The layout gives that initial terminal only the
   per-entry recipient token and canonical rail URL needed to address its
   original rail; no standing config or global Codex setting changes.
2. In the terminal that will run Codex, the operator runs
   `target/zaphod watch-tab --server URL`. The command verifies its inherited
   Zellij session/pane from one native inventory, resolves that pane's stable
   tab, requires exactly one tiled non-suppressed rail with the injected
   canonical URL, binds the private socket, and proves token-bound recipient
   delivery before returning ready. It prints the watched pane, stable tab,
   original rail, and watcher PID for debugging. This is the watcher's only
   native pane inventory for its lifetime.
3. The operator starts Codex in that same terminal. The trusted checkout-local
   `SessionStart` hook derives the same socket from inherited session/pane and
   sends one bounded versioned envelope. `SubagentStart`, malformed input,
   wrong session/pane, and a missing socket are rejected and never become rows.
4. The watcher canonicalizes the accepted Codex UUID to `codex:<UUID>`, uses
   only AgentsView's exact `/api/v1/sessions/{id}` endpoint, maintains one
   in-memory registration and source projection, and delivers full leased
   snapshots through the captured stable-tab/recipient-token route. AgentsView
   `data_changed` refresh, heartbeat, and shutdown use no native pane query;
   acknowledged snapshots are the only post-ready full-delivery boundary.
5. The rail renders only that watched session. A click may execute the retained
   exact-pane focus only while the plugin's current manifest still binds that
   pane in this rail's tab, the current recipient is armed, and the watcher
   lease is current. A later valid top-level `SessionStart` in that terminal
   replaces the in-memory ID; `Stop` remains a turn state, not deregistration.
6. Killing the watcher or losing delivery expires its generation within the
   bounded lease. Removing, moving, or suppressing its terminal causes the
   plugin's next current `PaneUpdate` to clear the unbound projection and lease
   atomically; removing the rail removes the rendering/focus surface itself.
   Restarting `watch-tab` creates a new empty generation; no old ID is
   reconstructed. The watcher does not rediscover, retry, or prove those
   lifecycle changes. A silently orphaned daemon after terminal or rail loss
   is explicitly acceptable until the operator or test harness kills it.

The documented walking skeleton supports one initial watched terminal per
managed tab. A later pane is never auto-discovered; an expert may start a
separate watcher only when explicitly given the original immutable route
bundle. Automatic launch, route propagation, and multi-pane ergonomics belong
to the filed follow-up.

### Socket, authority, and lease contract

The runtime parent is a short user-private directory (`0700`). Socket identity
is `watch-tab-v1/<sha256(exact session NUL decimal pane)>.sock`; the socket is
`0600`, owned by the current UID, and protected by a single-live-listener
lock. A new watcher refuses a live endpoint. It may remove a stale socket only
after `lstat`, ownership/type checks, and a failed bounded connect. No payload,
timestamp, PID, CWD, or file from an earlier watcher grants authority.

The hook sends one complete, maximum-1-MiB envelope containing protocol,
inherited Zellij session, inherited pane ID, provider, and the unmodified
provider hook object. The watcher decodes the record atomically, rejects
unknown/duplicate fields and trailing bytes, and accepts only Codex
`SessionStart` `startup|resume` with a non-empty canonical UUID. The socket is
an ingress transport, not a store; no registration file is written.

Before ready, the watcher performs one native `list-panes` inventory with only
`--json --all --state --tab` and requires: its terminal pane exists exactly
once; the pane belongs to one stable tab; and exactly one original plugin pane
in that tab has the canonical URL and managed tiled shape. This startup probe
requests no command, geometry, CWD, scrollback, or process metadata. The
watcher then binds its private socket and proves current recipient-token
delivery. A startup inventory or recipient failure produces no ready watcher.

After ready, the captured tuple is never revalidated by the daemon. Exact
AgentsView fetches, acknowledged full snapshots, and unacknowledged lease
heartbeats use only the captured stable tab, injected recipient token, and
random watcher generation. There is no post-ready native inventory, no cleanup
probe, no empty-snapshot teardown attempt, and no retry or serialization layer.
On ordinary signal/source failure the watcher stops its source work, closes its
listener, removes its socket, and exits; on terminal or rail loss it may remain
orphaned until explicit process cleanup.

Idle renewal is different: a recipient- and generation-checked heartbeat runs
every 1.4 seconds without a native pane query. The plugin accepts it only for
the current exact rail and clears the whole session projection if any bound
pane is absent from its current tab manifest. A late heartbeat cannot
resurrect an expired generation.

Each full snapshot carries a random watcher-generation nonce and a short
lease. The plugin replaces its session projection atomically, accepts
heartbeats only through its current exact recipient/tab proof, clears an
unbound session on current `PaneUpdate`, and clears the generation when the
lease expires. This prevents a killed watcher, missing terminal, or stale
delivery from leaving an actionable row. After startup the plugin—not daemon
inventory—owns projection and focus fail-close; the watcher supplies only
identity, source data, generation, and lease renewal.

The post-ready lifecycle is intentionally small:

| Event | Plugin result | Daemon result | Native inventory after ready |
| --- | --- | --- | --- |
| trusted registration / `data_changed` | one row only if current manifest, recipient, and generation agree | exact fetch + acknowledged snapshot | `0` |
| heartbeat | same generation lease renewed; no new row created | unacknowledged pipe | `0` |
| terminal missing, moved, or suppressed | current `PaneUpdate` clears row and lease; focus is inert | may remain orphaned | `0` |
| original rail missing | plugin state and visible surface are gone | may remain orphaned | `0` |
| watcher or delivery lost | row and focus expire within 2.5 seconds | stopped, failed, or unreachable | `0` |
| explicit cleanup | no row or focus; no durable recovery state | harness/operator kills owned PID and removes socket | `0` |

### Frozen-head retain/delete inventory

Retain and adapt: canonical Codex hook decoding and `codex:<UUID>` identity;
exact AgentsView fetch and typed response bounds; `SessionEvent.pane_id`;
atomic snapshot replacement; `registered_session_pane`; exact click focus;
stable tab/recipient token delivery and acknowledgments; target URL checks;
AgentsView fixtures; same-CWD `1/1/0` and managed-lookalike tests; `af7860a`
safe stale-socket admission; and `e1ddbb1` complete repeated-ack parsing.

Delete from KJ: `AgentRegistryV1`, file/lock/rename/permission code, stale
pruning, registry timestamps and generations, registry-dir flags and shell
propagation, registry polling, restart rehydration, and their tests/docs.
Replace the automatic `subscribe` launch in direct entry with initial-terminal
route context plus the manual `watch-tab` readiness path. Delete every
post-ready `probeWatchTarget` call and cleanup native probe; do not replace
them with retries, cached inventory, serialization, a CLI broker, or task-91
architecture. Do not retain the trusted-rail shortcut that allowed an absent
original rail at startup.

## Acceptance criteria

### Offline (agent-reproducible)

**AC-O1 — manual startup resolves one exact live authority tuple.** In two
same-CWD managed tabs, a watcher started in each selected terminal resolves
exactly its inherited `{session, pane, stable tab, original rail}` and never
the other tab's same-WASM rail. Missing or duplicate terminal/rail state and a
watcher launched outside Zellij produce no ready watcher.

Verified by: a disposable tmux/Zellij harness independently reads native
single-line JSON objects, compares the watcher's ready tuple with exact IDs,
and asserts ready counts `1,1,0` for the two valid watchers plus one foreign
shell. No fixture field under watcher control supplies the expected IDs.

**AC-O2 — the private socket admits only the matching top-level start.** A
valid matching `SessionStart startup|resume` creates one in-memory canonical
ID. Missing watcher, wrong session/pane/socket, `SubagentStart`, malformed,
trailing, duplicate-field, empty-ID, and 1-MiB-plus-one records create zero
registrations and zero rows. A later valid start replaces, rather than adds to,
the one watched-pane mapping.

Verified by: process tests using external Codex-schema fixtures, independently
derived socket paths and mode/owner sentinels, full-record validation, and an
in-memory diagnostic query. The filesystem assertion permits a socket/lock and
logs but fails if any registration or registry generation exists.

**AC-O3 — exact projection yields cardinality `1,1,0` and exact focus.** Two
same-CWD watched tabs render only their own top-level sessions; an unregistered
historical session and real-shape child render nowhere. Each accepted ID is
fetched only through its exact endpoint, and clicking a row focuses its exact
watched pane from a same-CWD spare.

Verified by: the retained AgentsView request-log fixture, two rail screen
captures, literal mouse input, and independent native focus state. Expected
row/request counts are exactly one per top-level ID and zero for the child,
global list endpoint, and wrong tab.

**AC-O4 — plugin manifest and lease fail closed without daemon rediscovery.**
Closing, moving, or suppressing the watched terminal clears its exact session
row and lease on the next current `PaneUpdate`; focus is immediately a no-op.
Watcher or delivery loss clears the row within 2.5 seconds. Removing the rail
destroys its plugin state, including the lease and row/focus surface; any
surviving daemon delivery cannot itself render or focus. Closing a same-WASM
rail in another tab changes nothing. A restarted watcher begins with zero rows
until a new trusted start arrives. A daemon orphaned by terminal or rail loss
is explicitly acceptable and grants no authority merely by remaining alive.

Verified by: one plugin/native lifecycle matrix with monotonic timestamps,
current manifest fixtures, screens, focus state, and lease deadlines. It covers
terminal close/move/suppress, original-rail and bystander-rail removal, watcher
kill, restart-without-hook, exact one-row/no-unbound rendering, and explicit
owned-process cleanup; no assertion depends on prompt daemon self-termination.

**AC-O5 — post-ready native inventory count is exactly zero.** Startup uses
one bounded native inventory containing no command, geometry, CWD, scrollback,
or process metadata. From the ready signal through registration, AgentsView
`data_changed`, heartbeat, terminal close/move, rail loss, and cleanup, the
watcher issues zero `list-panes` or equivalent native inventory calls and zero
cleanup probes. Full snapshots use only token-bound acknowledged delivery;
heartbeats request no plugin output. Literal `Alt /` remains managed-only, and
queued `Alt p`/`Alt n` are delayed no more than 500 ms over a no-watcher control.

Verified by: an argv-recording real-boundary wrapper marks the ready epoch and
asserts pre-ready inventory count `1`, post-ready inventory/cleanup count `0`,
and only the expected pipe calls across the complete lifecycle matrix. Retained
managed-tab digests and disposable Zellij key-action timestamps independently
prove managed ownership and responsiveness.

**AC-O6 — failure and explicit cleanup leave no durable authority.** Exact-ID
404, mismatched response ID, SSE/source timeout, socket collision, malformed
startup inventory, acknowledgment loss, interruption, and normal watcher exit
leave no actionable row after manifest clearing or lease expiry and write no
registration or recovery record. The harness explicitly kills any accepted
orphan daemon and removes its socket, tmux server, Zellij session, temporary
profile/runtime root, and other owned process state. Prompt automatic teardown
after terminal or rail loss is not required.

Verified by: injected failures plus a manual-cleanup ledger recording owned
PIDs before/after cleanup, socket/process/native absence after that explicit
step, a forbidden-registration-file scan, and pre/post standing config/layout
hashes. The exact-session request log proves no list or inferred fallback was
attempted.

### Captain-live (only after AC-O1 through AC-O6)

**AC-I1 — the manual journey is usable and visibly tab-local.** The captain
creates two disposable same-CWD managed tabs, manually starts one watcher in
each chosen terminal, launches one real Codex session per terminal, and asks
one to spawn a subagent. Each top-level ID appears exactly once in its own rail,
with no `unbound` label; the child/history appear nowhere; and each row focuses
its originating pane. Moving/closing one watched terminal clears only its row
through the manifest. Killing one watcher removes only its row within 2.5
seconds; restarting the watcher does not resurrect it until Codex emits another
trusted start. The operator may explicitly kill an orphaned watcher during
cleanup rather than waiting for automatic lifecycle teardown.

Verified by: captain observation plus watcher ready tuples, captured hook IDs,
exact AgentsView responses, two screen/focus states, kill/restart timestamps,
the instrumented zero-post-ready-inventory count, an explicit process-cleanup
record, and pre/post standing-state hashes. Any need to consult CWD, a registry,
an old hook event, or a post-ready native inventory fails the demo.

## Test plan

1. **Riskiest mechanism first.** Instrument the real watcher/Zellij command
   boundary, mark the ready signal, and run registration, exact projection,
   multiple AgentsView `data_changed` refreshes, heartbeats, terminal move and
   close, original- and bystander-rail loss, watcher kill, and explicit cleanup.
   Require pre-ready native inventory count `1`, post-ready inventory/cleanup
   count `0`, exactly one bound row with no `unbound`, manifest/lease removal,
   and a clean manual process ledger. Stop if any post-ready inventory is needed.
2. Write the failing watcher tests that expose current post-ready
   `probeWatchTarget` and cleanup probes, then remove only those calls. Preserve
   one exact startup resolution, private socket admission, `af7860a` safe stale
   recovery, and `e1ddbb1` complete repeated-ack validation.
3. Extend exact AgentsView projection tests for one in-memory registration and
   generation across repeated `data_changed` refreshes. Add exact
   404/mismatch/source/ack failures without retry, list fallback, or inventory.
4. Extend the existing Rust `PaneUpdate`, recipient, generation, snapshot,
   heartbeat, lease-expiry, and click tests with terminal close/move/suppress and
   rail-loss matrices. Ask how each test could pass while a stale row remained
   actionable; assert the exact row count, bound label, and focus no-op.
5. Update the two-tab native smoke around manual watcher commands and exact
   `1/1/0`; prove no historical/child/unbound rows, exact focus, terminal and
   rail visible-authority loss, watcher lease expiry, empty restart, responsive
   pane/tab keys, and explicit orphan cleanup. Never require daemon self-exit.
6. Run Rust/Go/build, hook, entry, managed-tab, subscription, two-rail,
   lifecycle, latency, and `diff --check` packets. No Zellij case may skip,
   touch `WORK`, mutate standing KDL, or introduce retry/broker/registry/
   supervisor/task-91 architecture.
7. Only after offline green, run AC-I1 with real Codex and isolated AgentsView;
   preserve exact IDs, row/focus screens, zero post-ready inventory evidence,
   lease timings, and the explicit removal of every disposable root and process.

## Documentation change

- README's session-row and fresh-tab sections must say that direct entry creates
  the managed tab but the operator explicitly runs `target/zaphod watch-tab`
  in the agent terminal before starting Codex. Document the one-time startup
  tuple, zero post-ready native inventory, plugin-manifest/lease fail-close,
  one-terminal limit, 2.5-second expiry, explicit orphan cleanup, and restart
  re-registration.
- Rewrite `docs/zellij-agentsview-live-demo.md` as the exact manual two-tab
  journey, including missing-watcher, child/history, kill, restart-empty, and
  exact-focus observations; remove registry inspection and rehydration claims.
- Update `docs/zellij-tmux-smoke-harness.md` and `grout/README.md` for socket,
  in-memory registration, leased projection, authority-loss, and no-registry
  evidence. Keep the archived CWD prototype historical.

## Out of scope

Product changes during ideation; automatic watcher launch; automatic discovery
or route propagation to later panes; multiple watched agents per watcher;
durable registry/state; registration recovery after watcher/plugin/sidecar
restart; same-name Zellij incarnation and pane-ID reuse recovery; inferring
identity from CWD/title/time/prompt/newest; global Codex hook installation;
non-Codex providers; standing Zellij mutation; gate behavior; and the broader
hub/controller architecture. Post-ready native retry, inventory caching, a CLI
broker or serialization queue, prompt daemon teardown after pane/rail loss, and
task-91 nonblocking metadata architecture are also excluded. Automation,
multi-pane support, orphan cleanup, durability, incarnation, and rehydration
remain in filed follow-up
`tab-local-agent-watcher-automation.md` (`6s0s2704zrms3med9n04mm4y`).

## Stage Report: ideation (cycle 2)

- DONE: Run the smallest real-harness spike first: obtain authoritative AgentsView session ID plus ZELLIJ_PANE_ID, register them, and prove exact isolation across two same-CWD managed tabs with spawned subagents absent—or return the failed mechanism probe without inference fallback.
  The disposable Zellij/tmux/Codex/AgentsView spike registered two exact `codex:<hook UUID>` IDs to panes 0 and 1, rendered one row per tab, and excluded one real spawned child; exact IDs and counts are recorded above.
- DONE: Reframe KJ so stable recipient routing and explicit session-to-live-pane registration jointly define the tab boundary; CWD, timing, titles, prompts, and newest-session selection are never authority.
  The revised contract reuses the Herdr/Superset/cmux hook-time join, chooses a native lock-safe ephemeral registry owned by `zaphod register-agent-session`, and requires fresh native pane membership before exact-ID fetch and delivery.
- DONE: Specify exact-cardinality, negative, lifecycle, cleanup, and restart/rehydration acceptance evidence while preserving KJ's existing managed-tab safety constraints and keeping implementation out of ideation.
  AC-O1 through AC-O6 and AC-I1 cover `1/1/0` cardinality, conflict/stale/foreign negatives, move/close/end/restart, atomic cleanup, V3 lookalikes, and real captain proof without authorizing product changes in this stage.
- DONE: Recover and compare the original Zaphod evidence and primary-source mechanisms used by Superset, Herdr, and cmux.
  The original spike/live demo proved only CWD+marker behavior; local Spaceterm notes and raw sources show all three comparators joining inherited terminal identity with hook-supplied agent session identity, while cmux's inference fallbacks are explicitly rejected here.

### Summary

Cycle 2 replaces KJ's CWD heuristic with a passed, field-proven hook-time
identity bridge. Stable recipient routing answers “which rail,” explicit
registration answers “which session and pane,” and fresh native membership
joins them; the revised packet measures exact cardinality, negative evidence,
lifecycle, rehydration, and cleanup while leaving implementation to the next
stage.

## Stage Report: ideation (cycle 3)

- DONE: Canonical Problem, approach, AC, test-plan, and out-of-scope sections contain the accepted cycle-2 identity contract with no competing stale contract.
  The CWD-bound cycle-1 body now sits under an explicit superseded historical heading, while the accepted identity bridge owns the sole canonical section spine.
- DONE: The passed real-harness spike and all feedback history remain intact without reopening the chosen hook-time registration mechanism.
  The real Codex/AgentsView `1, 1, 0` spike, primary-source comparison, and both captain rejection records remain verbatim in the normalized body.
- DONE: `spacedock status --read managed-tab-safety-session-integration --ac-scan` discovers authoritative AC-O1 through AC-O6 and AC-I1.
  The live scanner reports all seven accepted criteria from the canonical acceptance section and no superseded cycle-1 criterion.

### Summary

Cycle 3 changes structure only. It makes the accepted explicit
session-to-live-pane registration contract authoritative and machine-visible,
while retaining the passed mechanism evidence and complete feedback history.

## Stage Report: implementation

- DONE: Ship the lock-safe native SessionStart registrar with exact provider-to-AgentsView identity, atomic single-owner records, and fail-closed malformed/conflicting/stale behavior.
  Commit `2fa8e8424d196465cd00bd091932a65d4ef01107` contains the bounded registrar, canonical `codex:<UUID>` identity, private atomic registry, conflict suppression, and stale-record pruning.
- FAILED: Replace CWD admission and binding with exact registered pane membership through sidecar delivery, rendering, focus, move/close/restart lifecycle, and same-CWD child-negative coverage.
  Exact membership and `1/1/0` isolation work, but trusted-rail authority, later-pane registry propagation, and transient-inventory pruning remain unresolved MUST FIX NOW findings.
- FAILED: Prove cardinalities 1/1/0 and bounded cleanup in the disposable native harness, retain managed-tab safety suites, update user-visible docs, and obtain passing exact-head authoritative review.
  The harness, retained suites, and docs are green, but all three authoritative `code_completion` synthesis parents failed; no passing exact-head panel exists.
- SKIPPED: None.
  No dispatched checklist item was intentionally omitted; failed items are recorded as unresolved rather than deferred silently.

### Frozen candidate and green evidence

- Frozen clean product head: `2fa8e8424d196465cd00bd091932a65d4ef01107`; no product change followed the convergence stop.
- Rust baseline/current: 136/137 tests; `cargo test -q` passed 137/137 and `cargo check --tests` passed.
- `go test -count=1 -timeout 60s ./...` and `go vet ./...` passed.
- `tests/build-artifact-test.sh`, `tests/codex-session-hook-test.sh` from a clean detached checkout, and `tests/zellij-new-tab-test.sh` passed.
- Foreground `tests/zellij-tmux-smoke-test.sh` and `tests/zellij-two-rail-recipient-smoke-test.sh` passed; the latter proved shared-token same-CWD `1/1/0`, exact-pane click, restart rehydration, and native-close pruning.
- `git diff --check` passed, and exact-tip quick parent 1213/member 1212 returned P.
- TDD reds covered missing registrar and exact-projection behavior, over-limit input, path replacement, 404/prune/lock races, polling refresh, and registry-root propagation before their corresponding greens.

### Authoritative review rounds

- Parent 1175 reviewed `a5fc0f3649ac903bc45c737f188e50808a101a71..7e44833cb544c31c45ae2fc094ef0f6e288ae32d`; correctness 1172 F, journey 1173 F, proof 1174 F, synthesis F.
  Fixed 404 termination, prune race, late lock success, distinct-token acceptance, native-focus proof, clean-checkout hook build, and demo PID typo; session-generation authority was rebutted as excluded by the accepted contract.
- Parent 1191 reviewed `a5fc0f3649ac903bc45c737f188e50808a101a71..400e32b7e8d88ab7ee67a5612a8d26e6c0a79e3f`; correctness 1188 F, journey 1189 F, proof 1190 F, synthesis F.
  Fixed registry/pane-move refresh, initial managed-shell registry-root propagation, and invalid native-focus construction; session-generation authority repeated.
- Parent 1217 reviewed `a5fc0f3649ac903bc45c737f188e50808a101a71..2fa8e8424d196465cd00bd091932a65d4ef01107`; correctness 1214 F, journey 1215 F, proof 1216 F, synthesis F.
  Its six surviving findings are dispositioned below; no fourth panel was launched.

### Convergence gate

- MUST FIX NOW: continuously verify trusted rail identity instead of accepting `resident == 0`; this is an authority boundary.
- MUST FIX NOW: make native request-count assertions deterministic under the two-second poll; this is a test-only correction.
- MUST FIX NOW: propagate the registry directory to panes created after the initial managed shell; use a secure session-scoped handoff.
- MUST FIX NOW: perform the outside-Zellij no-op before checking for the product binary, with wrapper regression coverage.
- MUST FIX NOW: require repeated absence, a grace window, or a tombstone before pruning on incomplete native inventory.
- NEEDS DECISION: bind registry records to a Zellij session generation, or explicitly accept same-name session/pane-ID reuse; the accepted ideation contract placed durable incarnation recovery out of scope, while every panel treated it as blocking authority risk.
- Three failed synthesis parents (1175, 1191, 1217) exhausted the review-round budget. Work stopped at the frozen head for captain disposition; none of these authority, lifecycle, or proof findings was silently deferred.

### Summary

Implementation established the native registration and exact pane-membership walking skeleton and made the broad verification packet green. The stage is not complete: authoritative review failed three times, leaving five MUST FIX NOW findings and one contract-level session-generation decision for the captain before another implementation/review cycle.

## Stage Report: ideation (cycle 4)

- DONE: Rewrite KJ's canonical contract around one manually launched tab-local watcher and one watched agent terminal, with an exact operator journey and fail-closed authority lifecycle.
  AC-O1, AC-O3, AC-O4, and AC-O5 make the live watcher—not a file registry—the sole authority and cover startup, exact projection/focus, lease expiry, loss, managed ownership, and bounded host calls.
- DONE: Specify the private hook-to-daemon socket identity, exact original rail and pane checks, in-memory registration, and the smallest invalidating spike before implementation resumes.
  AC-O2 is backed by the passed disposable spike: missing daemon closed, a session/pane-derived mode-`0600` socket accepted one exact start, bystander rail loss was inert, and original rail loss terminated authority.
- DONE: Inventory frozen-head changes into retain/delete categories and move automatic launch, multi-pane discovery, durable registry, session incarnation, and restart rehydration to the filed follow-up.
  AC-O6 keeps exact-ID and cleanup negatives while registry/prune/rehydration machinery is deleted from KJ and owned by follow-up `6s0s2704zrms3med9n04mm4y`.
- SKIPPED: Product implementation and review rerun.
  This dispatch is ideation-only; AC-I1 remains captain-live, product head stays frozen at `2fa8e8424d196465cd00bd091932a65d4ef01107`, and no fourth implementation panel was launched.

### Acceptance evidence map

- AC-O1 is exercised by independent native tuple comparison across two same-CWD tabs plus one foreign shell.
- AC-O2 is exercised by the complete socket-envelope matrix and forbidden-registration-file assertion.
- AC-O3 is exercised by exact request counts, two screen captures, literal mouse input, and native focus state.
- AC-O4 is exercised by watcher/pane/tab/original-rail loss, bystander control, lease timing, and restart-empty state.
- AC-O5 is exercised by retained managed-toggle smokes, recorded native argv, and key-latency control.
- AC-O6 is exercised by injected source/socket/native failures, process/path cleanup, and standing-file hashes.
- AC-I1 is reserved for the captain's real Codex/AgentsView two-tab manual journey after all offline criteria pass.

### Summary

Cycle 4 replaces KJ's persistent registry with the captain-approved manual tab-local watcher and records a passed invalidating spike for its exact live authority tuple. The design keeps the already-proved identity/projection/focus seams, adds leased fail-closed rendering, and moves every automatic or durable recovery concern to the filed follow-up without changing product code.

## Stage Report: implementation (cycle 2)

- DONE: Replace the persistent registry and automatic subscriber with one
  manually launched watcher whose authority is the inherited Zellij session,
  terminal pane, stable tab, original rail, private socket, and one in-memory
  top-level `SessionStart` registration.
  Commits `5497cdb..838da0c` implement atomic hook admission, the bounded private
  transport, exact live-tuple resolution, exact AgentsView lookup, acknowledged
  leased snapshots, readiness handoff, the manual CLI journey, and deletion of
  the automatic subscriber authority. No registry or rehydration state remains.
- DONE: Preserve exact tab-local projection and focus with fail-closed lifecycle
  behavior.
  The two-rail native packet proves same-CWD cardinality `1/1/0`, exact endpoint
  selection, exact watched-pane focus, bystander isolation, watcher restart
  empty, manifest-owned silent-loss clearing, and explicit best-effort daemon
  and socket cleanup. Rust tests prove atomic generation replacement, expiry,
  absent-pane clearing, late-heartbeat rejection, and focus denial.
- DONE: Correct AC-O5 at the exact blocking boundary without adding a second
  authority or recovery layer.
  Classification: NARROW FIX — OUTCOME DEFECT; affected value criterion AC-O5.
  Commit `97c2d5461421af686e47743daa3e97bdc20b620c` removes recurring native
  inventory probes from idle renewal and reuses the recipient/generation
  heartbeat plus the plugin's exact manifest and 2.5-second lease. Commit
  `dec685e8cf6efbc84ef2b26f60a81e4eda9aacf5` sets a 1.4-second heartbeat,
  retains 1.1 seconds of lease slack, adds a one-second delivery-delay matrix,
  and makes the responsive-proof refresh selector atomic. Startup,
  hook/data-change full delivery, and cleanup retain bounded native checks;
  silent daemon teardown remains non-authoritative and best effort per AC-O4.
- DONE: Update the operator and proof documentation for the accepted manual
  journey and lease ownership.
  README, the live-demo guide, the tmux harness guide, and `grout/README.md`
  describe watcher-before-Codex ordering, exact hook identity, empty restart,
  no idle native polling, the 1.4-second heartbeat, 2.5-second expiry, and the
  filed automation/recovery boundary.
- SKIPPED: AC-I1 captain-live observation.
  The implementation stage completed the agent-reproducible AC-O1 through
  AC-O6 packet. AC-I1 remains explicitly captain-live and does not gate this
  worker's offline implementation report.

### TDD and verification evidence

- Reds preceded the watcher registrar, socket transport, exact live authority,
  leased/acknowledged snapshots, manual lifecycle/CLI/readiness, outside-Zellij
  hook no-op, two-watcher native journey, heartbeat pacing, idle-poll removal,
  delayed-renewal slack, and atomic in-flight refresh selection.
- `go test ./...` and `go vet ./...` passed in `grout`.
- `cargo test -q` passed 145/145 and `cargo check --tests` passed.
- `tests/build-artifact-test.sh`, `tests/codex-session-hook-test.sh`,
  `tests/zellij-new-tab-test.sh`, `tests/zellij-responsive-proof-test.sh`, and
  `tests/sidebar-scrollback-docs-test.sh` passed.
- `tests/zellij-watcher-lifecycle-smoke-test.sh` passed for outside- and
  inside-Zellij callers. `tests/zellij-two-rail-recipient-smoke-test.sh` passed
  exact `1/1/0`, focus, restart-empty, manifest-loss, and cleanup assertions.
- `tests/zellij-sidebar-congestion-test.sh` passed the literal one-second
  action deadline and injected timeout cleanup. The four-case
  `tests/zellij-stress-evidence-test.sh` packet passed lifecycle failure,
  native-command hang, vanished startup, and inconclusive-cleanup cases.
- `git diff --check` passed. Repeated back-to-back native harness runs can
  exhaust disposable Zellij fixture readiness; each affected packet passed in
  isolation and no product assertion was weakened to mask that fixture-load
  behavior.

### Review evidence

- Exact-head quick panel parent 1411/member 1410 reviewed
  `dec685e8cf6efbc84ef2b26f60a81e4eda9aacf5` and returned P: “No issues
  found.” Its prior exact-head round at `97c2d54` identified insufficient lease
  slack; `dec685e` is the accepted correction and adds the delayed-delivery
  regression.
- The first authoritative `code_completion` launch on the standing Roborev
  daemon, job 1415, failed before review because the Codex runtime rejected the
  incompatible combination `features.multi_agent_v2` plus
  `agents.max_threads`. No product verdict was produced.
- A clean isolated Roborev v0.62.0 daemon used the same repository panel and a
  compatible temporary Codex profile without changing standing configuration.
  Its first panel surfaced two Medium claims that contradicted the approved
  contract: automatic watcher launch is explicitly deferred, and `Stop` or an
  idle/completed exact session is explicitly not deregistration. The response
  also noted that one member had failed to read the repository.
- Re-evaluation run `ecc48d63-e2cc-4074-9479-33d3122821b1` reviewed
  `main..dec685e`; correctness job 5, journey job 6, proof job 7, and synthesis
  parent 8 all returned P. The authoritative synthesis verdict was “No issues
  found.”

### Summary

Cycle-2 implementation delivers the captain-approved manual tab-local watcher
without the rejected persistent registry or automatic recovery machinery. The
exact session-to-pane join, stable recipient delivery, leased rendering, and
exact focus are green. Idle authority renewal no longer calls synchronous
native pane inventory, so the watcher does not multiply Zellij's slow pane
metadata path; plugin manifest loss or heartbeat expiry removes visible and
actionable authority while daemon cleanup remains explicitly best effort.

## Stage Report: validation

- DONE: Independently reproduce AC-O1 through AC-O6 from the frozen
  implementation head and map each criterion to concrete rerun evidence.
  At `dec685e8cf6efbc84ef2b26f60a81e4eda9aacf5`, Go, Rust, build, hook,
  entry, two-rail, lifecycle, congestion, and four stress packets passed;
  `gates/managed-tab-safety-session-integration-validation.md` maps each AC.
- DONE: Verify the stored exact-head code_completion packet, then run the
  required adversarial refutation audit on a throwaway checkout without
  changing the implementation worktree.
  Captain-requested replacement parent 4 covers `9343129..dec685e`, names
  `code_completion`, contains correctness 1, journey 2, and proof 3 once each
  at P/done, and is P; its raw JSON and restartable database are retained under
  `/tmp/kj-roborev-rerun`. Three throwaway mutations also went red at exact
  binding, atomic record validation, and stale-lease boundaries.
- DONE: Prepare the exact captain-live AC-I1 demo and a Subspace gate record
  that distinguishes offline proof from the captain's observation.
  `gates/managed-tab-safety-session-integration.md` validates as a Subspace v0
  brief and points to the exact fresh-session script; AC-I1 is explicitly
  PENDING until the captain reports the live result and emits the decision log.
- SKIPPED: Run or claim the AC-I1 captain-live observation.
  The workflow makes the captain the validator for this floating-TUI journey;
  this worker prepared the demo but did not substitute harness evidence for it.

### Summary

All six offline criteria pass independent validation. At the captain's
request, the ephemeral-review retention evidence defect was repaired with a
replacement exact-head panel whose exported parent JSON and restartable
database remain available; no product file or standing configuration changed.
The gate is ready for the captain's two-tab manual watcher demo, after which
its Subspace decision log will record the live outcome.

## Stage Report: implementation (cycle 3)

- DONE: Implement only the AC-O6/AC-I1 stale-socket restart-admission outcome
  defect on KJ worktree head `3f44626`.
  Commit `af7860a` changes only watcher socket admission and its focused tests;
  the manual watcher and in-memory authority design remain unchanged.
- DONE: Safely prove stale before removal; a live watcher remains exclusive
  and ambiguous ownership fails closed.
  Admission requires a mode-0600 current-UID socket, a failed 100 ms connect,
  and unchanged inode/mode before unlink; live, foreign, wrong-mode, and
  wrong-type endpoints are preserved.
- DONE: Add focused stale/live/ambiguous socket tests first and keep the Go
  package compiling.
  The initial stale-restart test failed with `watch socket already exists`;
  stale restart, live reachability, ambiguity, and bounded concurrent
  different-pane admission now pass for 50 consecutive focused runs.
- DONE: Run focused socket/lifecycle tests and the exact two-watcher
  regression.
  Full Go tests and vet pass; watcher lifecycle passes outside and inside;
  the two-rail packet passes `1/1/0`, exact focus, empty restart, lease expiry,
  and no idle native polling.
- DONE: Run Rust and build verification as proportional cross-language proof.
  `cargo test -q` passes 145/145, `cargo check --tests` passes, `build.sh`
  passes, and `git diff --check` is clean.
- FAILED: Produce a green congestion packet from the current host.
  The candidate failed the unchanged native `new-tab` one-second deadline
  three times; exact pre-fix `3f44626` content failed at the identical boundary,
  classifying this as an evidence defect outside the authorized socket fix.
- DONE: Return the commit plus a focused Roborev handoff.
  Review 1496 found the nonblocking shared-lock regression; `af7860a` added a
  bounded wait and deterministic different-pane test, review 1496 was closed,
  and replacement exact-head review 1499 returned `No issues found.`
- SKIPPED: Run or claim the captain-live AC-I1 observation.
  No captain Zellij session was mutated; the fresh two-tab drill remains the
  captain's next observation after the FO accepts the baseline congestion
  evidence classification.

### Summary

Cycle-6 feedback is implemented as the final narrow socket-admission repair:
a killed watcher's proven-stale endpoint no longer blocks a normal retry, while
a live or ambiguous endpoint cannot be evicted. All product-local and exact
watcher proofs are green at `af7860a`; only the congestion harness remains red
at the same pre-fix evidence boundary and is reported without scope expansion.

## Stage Report: implementation (cycle 4)

- DONE: Implement only the AC-O1/AC-I1 readiness-cardinality outcome defect
  on KJ head `af7860a`.
  Commit `52973da` changes only readiness acknowledgment parsing, immediate
  native rail cardinality, and their focused tests.
- DONE: Accept one or more complete concatenated `ready` atoms and reject
  empty, partial, or foreign output.
  The parser accepts single, double, and four-runtime acknowledgments while
  rejecting empty, truncated, prefixed, suffixed, spaced, and newline-split
  records atomically.
- DONE: Retain initial exact-one native rail resolution and strengthen
  immediate pre-delivery authority probing.
  `probeWatchTarget` now requires the original rail exactly once and exactly
  one total same-URL tiled, non-suppressed rail in the stable tab.
- DONE: Add focused duplicated-runtime readiness and duplicate-native-rail
  rejection tests first.
  Before the fix, `readyready` timed out and a second matching native rail
  preserved authority; the full watcher and both focused boundaries now pass.
- DONE: Preserve manual watcher, in-memory authority, and cycle-6 socket
  admission behavior.
  No registry, supervisor, alternate proof path, or socket-admission code
  changed; the pre-existing checkout-local hook edit remains uncommitted.
- DONE: Run focused Go/vet, Rust, two-rail, and lifecycle verification.
  Focused race tests, full Go tests and vet, Rust 145/145 plus check, build,
  two-rail `1/1/0`, and outside/inside lifecycle packets all pass.
- DONE: Run one exact-head focused Roborev review.
  Quick synthesis review 1555 at `52973da` returned `No issues found` and
  confirmed duplicated complete acknowledgments retain duplicate-pane
  fail-closed checks.
- DONE: Prepare the exact captain-live retest without mutating live Zellij.
  Disposable checkout `/tmp/zaphod-kj-standalone-5d25144` is clean, detached
  at `52973da`, and rebuilt; tabs 1 and 2 in `kj-final-1784089995` retain their
  existing route bundles and need only rerun `./target/zaphod watch-tab`.
- SKIPPED: Run or claim the captain-live AC-I1 observation.
  The worker sent no pipe and changed no live pane; readiness and one-row-per-
  tab projection remain the captain's observation.

### Summary

Cycle 7 now treats Zellij's repeated per-client outputs as one logical exact
rail acknowledgment without weakening physical rail cardinality. All requested
offline proof is green at `52973da`; the prepared live retest deliberately
exercises both stale-socket recovery and `readyready` acceptance in place.

## Stage Report: ideation (cycle 5)

- DONE: Rewrite the canonical current contract around startup-only native
  authority.
  One pre-ready inventory resolves the exact terminal, stable tab, and original
  rail; the contract requires zero native inventory or cleanup probes afterward.
- DONE: Assign post-ready row and focus fail-close to existing plugin state.
  Current `PaneUpdate`, recipient proof, watcher generation, exact pane binding,
  and the 2.5-second lease now define every actionable projection state.
- DONE: Replace contradictory AC-O4, AC-O5, and AC-O6 lifecycle obligations.
  Terminal close/move/suppress clears through the manifest, rail/watcher loss
  removes or expires authority, and orphan daemons are accepted until explicit
  manual cleanup rather than required to self-terminate.
- DONE: Specify the riskiest implementation proof before further code changes.
  The instrumented matrix requires one pre-ready inventory, zero post-ready
  inventory across registration/data-change/heartbeat/loss/cleanup, exactly one
  bound row with no `unbound`, lease expiry, and a clean owned-PID ledger.
- DONE: Preserve accepted implementation inputs without preserving the failed
  recurring-probe mechanism.
  AC-O1, AC-O2, and AC-O3 retain the exact startup/socket/projection evidence;
  `af7860a` safe stale admission and `e1ddbb1` repeated-ack parsing remain
  inputs, while post-ready `probeWatchTarget` and cleanup probes do not.
- DONE: Bound the mechanism to the smallest walking skeleton.
  Retry, native-inventory caching, CLI broker/serialization, registry,
  supervisor, durable recovery, and task-91 architecture are explicitly out.
- DONE: Update the proposed documentation delta and captain-live journey.
  The docs must teach startup-only inventory, manifest/lease fail-close,
  exact one-row/no-unbound projection, and explicit orphan cleanup.
- SKIPPED: Modify or verify product code during ideation.
  AC-O4, AC-O5, and AC-O6 are intentionally unmet until the first implementation
  proof; product head `e1ddbb1` remains an input and no product file was changed.
- SKIPPED: Mutate or claim evidence from captain Zellij sessions.
  AC-I1 remains unmet; its live drill is gated behind the new offline
  zero-inventory proof.

### Summary

Cycle 8 removes the unstable post-ready native lifecycle controller while
keeping exact startup identity, token-bound delivery, manifest binding, and
lease expiry. The next implementation must first prove that this smaller
authority split stays exact and fail-closed with zero post-ready inventory.

## Stage Report: implementation (cycle 5)

- DONE: Implement the approved startup-only authority contract from `e1ddbb1` without retry, cache,
  broker, registry, supervisor, durable recovery, or task-91 architecture. Commits `f4873d0` and
  `bd18165` preserve `af7860a` and repeated-ack behavior while deleting post-ready/cleanup probes.
- DONE: Prove exactly one native inventory before ready and zero afterward across registration,
  repeated `data_changed`, heartbeat, close/move/suppress, rail loss, and explicit cleanup.
- DONE: Keep row and focus fail-close through current manifest state, exact registered pane,
  recipient, generation, and 2.5-second lease; silent pane/rail loss may leave an orphan daemon.
- DONE: Bind every ready/snapshot/heartbeat delivery to the startup-resolved original rail ID.
  NARROW FIX — OUTCOME DEFECT at AC-O4/AC-O5: a replacement same-token rail could accept the
  orphan watcher; `bd18165` now requires `recipient-rail-id` to equal the receiver plugin ID.
- DONE: Repair the two-rail harness's inherited-client proof boundary.
  NARROW FIX — EVIDENCE DEFECT: inherited `ZELLIJ`/session/pane overrode the isolated route and
  produced “There is no active session”; the harness now clears invoking-client identity.
- DONE: Update operator/grout docs for startup-only inventory, manifest/lease fail-close,
  explicit orphan cleanup, and exact original-rail delivery identity.
- SKIPPED: Run or claim the captain-live AC-I1 observation.
  No captain session was mutated; the required worker-owned retained-client proof was isolated.
- FAILED: None. Every authorized implementation and proof item completed.

### TDD and verification evidence

- RED: native counts were `2` not `1` for one watcher and `6` not `2` for two; cleanup added a
  call. Final tests hold `1`, `2`, and no cleanup increment. Rail REDs omitted ID `50` and accepted
  replacement ID `51`; both pass after `bd18165`.
- `go test ./... -count=1`, `go vet ./...`, Rust 147/147, `cargo check --tests`, `build.sh`, and
  `git diff --check` pass. Two-rail `1/1/0` and outside/inside lifecycle smokes pass.
- An ordinary detached checkout at exact final SHA
  `bd181658e06b917964e5697580cd0d198266ac99` passed real AgentsView evidence at
  `/tmp/kj-retained-bd18165.WZojCX/evidence`: clients `1+3` became `3`; one `list-panes`, four
  live `data_changed`, six snapshots, 22 quiet seconds, live PID/socket, one bound `codex`, zero
  `unbound`, and no surviving isolated session.
- Quick 1644 found the rail gap and was fixed, annotated, and closed. Exact-head quick 1649 passed;
  three-reviewer `code_completion` 1653 returned “No issues found.”
- The branch is pushed through `bd18165`. The pre-existing `.codex/hooks.json` edit remains
  uncommitted and untouched.

### Summary

Cycle 9 completes the startup-only walking skeleton: one admission inventory, exact
recipient/tab/original-rail/generation/lease projection afterward, and UI fail-close without the
blocking metadata controller. Offline, native, retained-client, and review evidence is green.
