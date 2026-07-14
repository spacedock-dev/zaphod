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

## Problem

This contract supersedes the cycle-1 CWD-bound journey. The previous merge
shape and managed-tab proof remain constraints, but CWD is no longer a
session-routing input.

### Identity gap

BB proved that a private sidecar can deliver to one stable tab and V3 proved
which rail may own managed actions. Neither proves which live terminal
originated an AgentsView session. Current production code first lists up to
1,000 sessions with `include_children=true`, selects every session whose CWD
appears in the recipient tab, then lets the plugin bind that CWD to one pane.
Two same-checkout tabs therefore admit the same history; child sessions can
also pass through when AgentsView's child metadata is incomplete.

The original Zaphod evidence did not contain a stronger join:

- The archived 2026-07-07 AgentsView spike explicitly chose “identity binding
  lives in the plugin” through exact CWD matching
  (`docs/archive/plan-agent-rail-prototype-2026-07-07.md`, shipped prototype
  decision 1).
- The later live packet found a session by CWD plus unique first-message text
  (`docs/zellij-agentsview-live-demo.md`, steps 3–6). It proved SSE refresh,
  not session-to-pane identity.
- The 2026-07-08 child filter (`e3619bb`) recorded that AgentsView's own child
  and automated flags missed 59/59 sampled Task-tool children and substituted
  an `agent-` ID-prefix heuristic. The current Codex child observed below has
  a normal `codex:<UUID>` ID, so that historical heuristic is not authority.
- `zellij-managed-identity-feasibility.md` proves a possible managed-view
  marker and session-incarnation direction. It never joins an AgentsView
  session ID to the terminal that launched it.

## Primary-source comparison and reused mechanism

The field already supplies the mechanism; KJ should reuse it rather than
inventing another identity inference.

- **Herdr** injects `HERDR_PANE_ID`, `HERDR_SOCKET_PATH`, and `HERDR_ENV` into
  each terminal. Its agent hook reads the agent-native session reference from
  hook stdin and reports it with that inherited pane ID. The server stores the
  hook authority on the pane and persists the session reference. See
  `spacedock-ui-research/notes/herdr-agent-integration.md`,
  `herdr/src/integration/mod.rs`, `herdr/src/cli/pane.rs`, and
  `herdr/src/terminal/state.rs` in the local Spaceterm research checkout.
- **Superset** injects `SUPERSET_TERMINAL_ID` into the PTY
  (`packages/host-service/src/terminal/env.ts`), extracts the agent's
  `session_id` from lifecycle-hook JSON, posts both values, and keys its live
  `TerminalAgentStore` by terminal ID. Terminal exit deletes the binding; a
  new session ID replaces the prior session in that terminal. See
  `notify-hook.template.sh` and `packages/host-service/src/terminal-agents/`.
- **cmux** receives an agent-native hook session ID alongside inherited
  `CMUX_WORKSPACE_ID`/`CMUX_SURFACE_ID`, validates that the direct surface is
  still accessible, and persists `sessionId → {workspaceId, surfaceId}` for
  hook and restart routing (`CLI/CMUXCLI+AgentHookDefinitions.swift` and the
  `ClaudeHookSessionStore` in `CLI/cmux.swift`). cmux also has TTY/PID and
  newest-active recovery paths for agents that strip environment. KJ does not
  reuse those fallbacks: an absent direct join must stay absent.

The common proven shape is **terminal identity inherited from the spawn
context + agent session identity supplied by the lifecycle hook**. Zellij
already provides the two required terminal values to processes in a pane:
`ZELLIJ_SESSION_NAME` and `ZELLIJ_PANE_ID`. Codex 0.144.1's documented
`SessionStart` hook input supplies `session_id`; `SubagentStart` is a distinct
event and carries the parent session ID. Those are the only identity sources
authorized here.

The wider research agrees with this choice:
`spacedock-ui-research/problem-map.md` names env-contract pane↔session binding
as solved field work to borrow, and `capability-matrix.md` records hook-bound
session awareness in Herdr, cmux, and Superset while identifying Zaphod's gap.

## Riskiest mechanism spike — run first, PASSED

The smallest real harness spike ran on 2026-07-14 with Zellij 0.44.3, tmux
3.6a, Codex 0.144.1, and AgentsView 0.37.5. It used disposable
config/data/socket/HOME/Codex/AgentsView roots and the current candidate WASM.

1. It opened two rail-bearing tabs, `Managed-A` and `Managed-B`, at the exact
   same `/Users/clkao/git/zaphod` CWD. Native state assigned tab/pane pairs
   `0/0` and `1/1`.
2. A real Codex process started through each attached tmux/Zellij terminal.
   `SessionStart` hook stdin supplied
   `019f5f99-38d9-7352-824e-e42734bf8d9f` in pane 0 and
   `019f5f99-39be-7af0-94b5-de0da85f96d6` in pane 1. The inherited
   `ZELLIJ_SESSION_NAME` was `kj-spike-32122` for both.
3. Isolated AgentsView indexed those exact sessions as
   `codex:019f5f99-38d9-7352-824e-e42734bf8d9f` and
   `codex:019f5f99-39be-7af0-94b5-de0da85f96d6`. Thus the Codex adapter's
   canonical AgentsView key is the observed `codex:` namespace plus the exact
   hook UUID; a failure of that exact lookup is terminal.
4. The second Codex process spawned one real subagent. Codex emitted one
   `SubagentStart` with the parent ID and wrote a separate child transcript;
   AgentsView indexed the child as
   `codex:019f5f99-62ef-7661-9e76-3a1f1a93d351` but did not label it as a
   child in list output. Because it produced no top-level `SessionStart`
   registration, the explicit registry omitted it.
5. The registry-to-live-pane join projected exactly one row per tab. Stable
   recipient-token pipes rendered `KJ_TAB_A_ROW` only in A and
   `KJ_TAB_B_ROW` only in B; the unregistered child rendered nowhere. The
   tmux server and Zellij session were removed afterward.

This passes the mechanism that CWD could not prove. It also sharpens the
failure rule: AgentsView classification is not sufficient to exclude a child;
only an exact top-level registration admits a session. If a future provider's
startup hook cannot supply an agent-native ID that resolves to one exact
AgentsView ID, that provider is unsupported for this slice. CWD, timing,
titles, prompts, ID prefixes, and newest-session selection remain forbidden.

## Proposed approach

### One native, lock-safe ephemeral registry

Add a native `zaphod register-agent-session` hook receiver and a versioned,
user-private registry scoped by the exact Zellij session. The native binary,
not a shell hook or WASM plugin, owns locking, validation, atomic replacement,
and garbage collection. Its minimum record is:

```text
AgentPaneRegistrationV1 {
  zellij_session: exact inherited ZELLIJ_SESSION_NAME,
  pane_id: exact inherited ZELLIJ_PANE_ID,
  agent: provider namespace, initially "codex",
  agent_session_id: exact provider SessionStart id,
  agentsview_session_id: provider adapter's exact canonical key,
  pid: hook caller/process observation for lifecycle diagnostics only,
  updated_at: monotonic/UTC registration observation,
}
```

The file lives under a per-user runtime directory with a `0700` parent and
`0600` contents. The receiver takes a native advisory lock, validates one
JSON hook object, writes a same-directory temporary file, fsyncs, and renames.
It accepts only `SessionStart` `startup|resume` for registration. A duplicate
pair is idempotent; a new top-level session in the same live pane supersedes
the old one. The same session ID concurrently claimed by two live panes is an
ambiguous conflict and neither claim is deliverable. `pid` and `updated_at`
help cleanup but never override the exact session/pane pair.

For the selected-checkout dogfood slice, a trusted repo-local Codex hook calls
the checkout-built native receiver. It exits successfully without mutation
when the two Zellij environment values are absent. General installation,
other agents, and a portable provider registry remain later work. Hook trust
is explicit; implementation must not modify the captain's global Codex config.

### Stable recipient plus fresh pane membership defines the tab

The direct entry still creates one V3-proved rail and starts exactly one
private sidecar with its stable tab ID, canonical rail URL, and recipient
token. On startup and every source refresh, that sidecar:

1. reads one consistent registry snapshot;
2. queries native `list-panes` for the exact Zellij session;
3. retains only registrations whose terminal pane is live and currently a
   member of its stable tab ID;
4. fetches only each retained `agentsview_session_id` through AgentsView's
   exact `/api/v1/sessions/{id}` endpoint; and
5. emits a session snapshot carrying the registered `pane_id` through the
   existing stable-tab/recipient-token channel.

The plugin adds `pane_id` to `SessionEvent`. Rendering treats a row as bound
only when that exact terminal ID is present in the rail's current tab manifest;
clicking focuses that ID directly. The CWD fields may remain display metadata
but leave `bind_session`, admission, focus, and ordering. Extend the existing
pure `apply_agent_snapshot`, `apply_agent_event`, `section_layout`, and
`decide_rail_click` functions; replace the CWD-only pure `bind_session` with an
exact `registered_session_pane` membership decision. No title, prompt, clock,
or list order enters the decision.

The source and delivery probes bracket the exact fetch. A pane moved between
the two managed tabs therefore yields zero or one row during convergence and
then exactly one row in its new tab—never one in each. Closing the pane removes
the row. A Codex `Stop` is a turn boundary, not deregistration; the row remains
and AgentsView supplies its updated state. A later top-level `SessionStart` in
the same pane replaces the prior session. Plugin or sidecar restart rehydrates
from the registry, then revalidates native membership before redelivery.

This cycle does not claim safety across destruction and recreation of a whole
same-named Zellij session with reused pane IDs; durable session incarnation is
owned by `zellij-managed-identity-feasibility`. Until that capability lands,
full native-session replacement must discard the ephemeral registry rather
than rebind it.

## Acceptance criteria

### Offline (agent-reproducible)

**AC-O1 — registration is an exact, single-owner identity bridge.** Two valid
top-level `SessionStart` hook objects produce two records whose canonical
AgentsView IDs resolve exactly and whose pane IDs are live in the named
Zellij session. Duplicate input is idempotent; malformed, missing-ID,
non-Zellij, non-start, and conflicting live claims produce no deliverable
record. No CWD, title, prompt, timestamp proximity, ID prefix, or list order is
consulted.

Verified by: native registrar table tests and a process test with an external
hook JSON fixture, independently created registry directory/mode sentinels,
concurrent writers, injected crash-before-rename, and exact AgentsView fixture
lookups. The failure fixture deliberately makes CWD/title/time/newest all point
at the wrong session and still expects zero rows.

**AC-O2 — each same-CWD managed tab has exactly one top-level session and zero child rows.**
Two managed tabs share one checkout CWD;
their distinct terminal panes register distinct top-level session IDs. Each
rail renders exactly its registered row and never the other's. One extra
AgentsView session plus `SubagentStart` evidence renders in neither rail.

Verified by: productionizing the passed tmux/Zellij spike with deterministic
Codex-schema hook fixtures and an AgentsView-compatible exact-ID server. Native
tab/pane inventory, registry JSON, exact HTTP request log, targeted pipe acks,
and both screen captures independently assert cardinalities `1, 1, 0`. The
already-run real Codex/AgentsView spike above is retained as feasibility
evidence; CI does not require network credentials.

**AC-O3 — stale, ambiguous, unregistered, and foreign-tab sessions fail closed.**
A closed pane, a nonexistent pane, a registration for another
Zellij session, one session concurrently claimed by two live panes, an
unregistered historical top-level session, and an unregistered child each
produce zero rows and zero focus actions. A same-CWD, same-title, same-agent
lookalike cannot change any result.

Verified by: a native-state/registry/HTTP fixture matrix plus Rust snapshot and
click tests. Every negative records an exact zero row count, zero pipe for the
rejected ID, and `ClickAction::None`; the HTTP log proves no rejected ID was
silently replaced by a list-selected session.

**AC-O4 — lifecycle and rehydration preserve exact cardinality.** Moving one
registered pane A→B removes its row from A and yields exactly one row in B;
closing it yields zero rows. A `Stop` event retains the same mapping; a new
top-level start in that pane replaces the old ID. Killing and restarting the
plugin, the sidecar, or both rehydrates the surviving mapping to exactly one
row with no duplicate and no need for another hook event.

Verified by: one disposable native sequence with before/after pane membership,
registry generations, exact-ID HTTP logs, session snapshots, pipe acks, and
screen/focus projections at each transition. A barrier moves/closes the pane
between source fetch and delivery to prove the second membership probe removes
the stale result.

**AC-O5 — managed-tab ownership remains intact.** The direct entry starts one
registrar-aware private sidecar only after the existing resident/URL/marker
proof. Stable recipient ID/token still admits the snapshot to one rail.
Literal `Alt /` changes only the V3-proved managed rail; a same-WASM lookalike
without the marker remains byte-identical and receives no session row.

Verified by: the combined disposable smoke plus retained
`zellij-new-tab-test.sh`, `zellij-tmux-smoke-test.sh`, and
`zellij-two-rail-recipient-smoke-test.sh`. Native pane/tab/layout/focus
projections prove both the positive and lookalike negative.

**AC-O6 — failure and cleanup are bounded.** Hook-ID-to-AgentsView-ID mismatch,
registry corruption, source timeout, sidecar kill, and harness interruption
leave no guessed row, orphan sidecar/hook helper, tmux server, Zellij session,
temporary registry/profile/Codex/AgentsView root, or standing config/layout
change. A failed exact lookup is terminal and visible in sidecar evidence.

Verified by: injected failures under existing harness traps, bounded process
and native-session absence checks, registry-root removal, and pre/post
standing-file state digests. The harness owns every temporary process and path;
normal product hooks remain fire-and-forget and bounded.

### Captain-live (only after AC-O1 through AC-O6)

**AC-I1 — real Codex sessions follow their panes, not their checkout.** In the
disposable managed profile, the captain starts one real Codex session in each
of two same-CWD tabs and asks the second to spawn one subagent. The two
top-level AgentsView IDs appear exactly once in their originating rails; the
child and existing history appear nowhere. Clicking each row focuses its exact
registered pane. Moving one pane moves its row, and restarting that tab's
sidecar restores one row without another prompt or hook event.

Verified by: captain observation plus captured hook payload IDs, registry,
native pane/tab IDs, exact AgentsView responses, row screens, focus state, and
pre/post standing-state hashes. Any failure to correlate the real hook ID with
one exact AgentsView ID ends the demo; no inference fallback is permitted.

## Test plan

1. **Riskiest mechanism first — DONE, PASSED.** Preserve the real spike result
   above. Before product changes, reduce it to a focused registrar prototype
   that performs native lock-safe atomic upsert and exact-ID fetch. If the
   implementation cannot reproduce the hook UUID→`codex:<UUID>` join, stop
   with the failed probe and no CWD/timing/title/prompt/newest fallback.
2. Add registrar unit/process tests: permissions, schema, SessionStart-only
   admission, canonical provider ID, duplicate idempotence, same-pane
   replacement, concurrent live conflict, stale-pane cleanup, lock contention,
   corrupt input, and crash-before-rename. The expected hook shape comes from
   Codex's published hook schema, not a product-authored prose parser.
3. Extend the AgentsView fixture with `/api/v1/sessions/{id}` and a request
   log. Change the sidecar from global list+CWD filtering to registry snapshot
   + fresh native membership + exact-ID fetch + registered pane delivery.
   Test not-found, mismatched returned ID, timeout, and move/close races.
4. Extend `SessionEvent` with `pane_id` and replace CWD binding in render/click
   decisions. Run pure tests for exact live membership, zero/duplicate IDs,
   snapshot removal, click focus, and every AC-O3 lookalike.
5. Productionize the two-tab tmux smoke using deterministic hook/API fixtures;
   then add the pane move, close, new-session replacement, plugin restart,
   sidecar restart, stale-delivery barrier, and cleanup cases. Keep real PTY
   input, independently bounded native calls, unique roots, and exact counts.
6. Run Rust/Go/native entry and retained managed-tab suites, then the full
   relevant shell packet and `git diff --check`. No Zellij case may silently
   skip; no test may touch `WORK` or standing KDL.
7. Only after offline green, run AC-I1 with real Codex and isolated AgentsView.
   Preserve IDs and negative evidence, then delete the disposable registry,
   AgentsView/Codex roots, Zellij session, and tmux server.

## Documentation change

- Rewrite README's “Tab-bound session rows” and fresh-managed-tab section to
  say that stable recipient routing limits delivery while explicit
  SessionStart registration authorizes the exact session and live pane. State
  that CWD/title/prompt/time/newest and child classification do not bind.
- Replace the CWD-marker proof in `docs/zellij-agentsview-live-demo.md` with
  the exact two-tab registration/cardinality/move/restart demo above.
- Extend `docs/zellij-tmux-smoke-harness.md` with the registry, exact-ID HTTP,
  move/close, child-negative, and restart evidence surfaces.
- Keep the archived prototype record historical; add no claim that its CWD
  decision was authoritative product architecture.

## Out of scope

Implementing the registrar, sidecar, plugin, hook asset, or tests in ideation;
global Codex hook installation or mutation; non-Codex provider adapters;
inferring identity for a provider without an authoritative startup ID;
whole-machine session adoption; durable same-name Zellij-session incarnation
and pane-ID-reuse recovery; multi-client routing; standing Zellij mutation;
gate discovery/review/resolution; and the broader hub/controller architecture.

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
