---
title: Integrate managed-tab safety with tab-bound session delivery
status: ideation
group: walking-skeleton
sprint: s1-managed-tab-safety
sprint-readiness: ready
score: 1.0
source: captain direction 2026-07-13; V3/BB merge conflict
id: kjhq0t2h6drse6b32cqybggv
started: 2026-07-13T06:54:16Z
---

## Problem

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

## Required outcome

An operator can use a selected-checkout, main-built fresh managed tab whose
exact `zaphod_managed_tab "v1"` plus canonical WASM URL authorizes its `Alt /`
route. The direct entry starts exactly one BB subscriber bound to that tab's
native stable ID; a session source update appears only in that rail and its
row focuses the one bound terminal. The managed rail toggles, while an active
same-WASM/sidebar-shaped tab without the proof remains inert. README describes
all of those limits together. This repair never activates, installs, rewrites,
or otherwise changes standing Zellij config or layout.

## Proposed approach

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

## Acceptance criteria

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

## Test plan

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

## Documentation change

Update only README's fresh-managed-tab section and real-key command block to
resolve the conflict as described above. Preserve the BB direct-entry and
stable-tab subscriber instructions, add the V3 ownership proof as a completed
constraint, and list the combined disposable smoke beside the two retained
smokes. Add one clear deferral sentence: gates are not emitted or pooled here;
their lifecycle remains S9/QT. Do not rewrite historical documents or turn the
evergreen architecture's later hub into a claim about this slice.

## Out of scope

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

## Stage Report: ideation

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

### Feedback Cycles

#### Cycle 1 — 2026-07-14 — captain rejected ideation

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
