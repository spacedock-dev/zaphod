---
id: 6vbb1n6rpv6qy0hs6z3avctx
title: Zellij managed identity and marker feasibility spike
status: ideation
source: managed-view roadmap Sprint 1 native feasibility lane, senior staff review 2026-07-11
started: 2026-07-11T05:25:14Z
completed:
verdict:
score: 0.98
worktree:
issue:
pr:
mod-block:
---

## Problem

Production controller work depends on unproved Zellij identity mechanisms: an exact multi-client key invocation witness, a durable marker owner and query path, a stable session-incarnation representation, and recovery across create-before-persist crashes and native ID reuse.

## Sprint role

Run a disposable, throwaway feasibility spike after the foreground-profile gate. Prove or reject invocation witness, persistent marker ownership/query, detach/reattach and replacement identity, stable-ID reuse, and crash-window recovery. Do not grow the spike into production controller code. Its gate either freezes the Zellij representation for integration with the binding core or returns the architecture for revision.

## Proposed approach

The spike tests one provisional representation rather than treating a tab ID or
name as ownership. `tab_id` remains a locator only. The candidate observation
is:

```text
SessionIdentityV1 {
  namespace: private Zellij data/socket namespace,
  native_session_id: exact Zellij session name,
  incarnation: random session nonce,
}
ManagedViewMarkerV1 {
  schema: "zaphod.binding.v1",
  binding_id: generated binding UUID,
  session_incarnation: same nonce,
}
```

The carrier is one test-only marker-controller plugin pane in the managed tab.
Its canonical local WASM URL plus the three configuration fields above are the
marker. The profile namespace scopes a server; it is not an incarnation. The
tab's display name is advisory only and does not participate in ownership.

The query path must be a fresh controller invocation, not a CLI action guessed
from the active client. The controller receives the actual temporary
`MessagePlugin` keybinding, obtains its host-resolved focused pane/tab, gets
the stable tab IDs from `TabUpdate`, and calls
`dump_session_layout_for_tab(tab_id)` for each candidate. It parses the live
KDL structurally and accepts only one pane with the exact controller URL and
the complete marker tuple. The harness independently captures
`action list-panes --json -a -g -t` to confirm the returned tab ID, plugin
pane, and URL. `list-panes` cannot substitute for the KDL query because its
`PaneInfo` exposes a plugin URL but not plugin configuration.

This design builds on, but does not overclaim, prototype evidence. The prior
CLI `Run` experiment mis-targeted or no-op'd because transient CLI clients had
no reliable active-tab entry; the architecture instead found an invoking pane
by mapping `ZELLIJ_PANE_ID` through `list-panes`
(`docs/zaphod-workspace-architecture.md`, “Zellij controller boundary”). The
live prototype also found that CLI active-tab actions follow the last
key-active client, whereas a plugin host action uses its attached client's
identity (`docs/docking-approach.md`, “Anomaly resolved”, 2026-07-04). Neither
observation proves a durable marker, cross-client marker query, session
incarnation, ID-reuse handling, or crash recovery; this spike supplies that
missing proof.

The existing `rail "1"` configuration demonstrates only URL-plus-
configuration plugin identity; it is static and not a per-binding marker
(`SPEC.md`, “Plugin lifecycle”). A crashed rail can also remain in the native
pane manifest, so resident presence cannot prove liveness or ownership
(`SPEC.md`, “State”). No prior drill intentionally demonstrated Zellij native
ID reuse, session replacement, or the native-create/portable-persist window.

### Evidence matrix

| Question | Disposable observation and oracle | Prove | Reject and recovery consequence |
| --- | --- | --- | --- |
| Exact multi-client invocation | Start two foreground PTY clients in one private Zellij 0.44.3 session. Give each a distinct, known foreign tab. Deliver the real temporary `Alt Shift z` binding through each PTY, then concurrently through both. The controller writes a test-owned `InvocationWitnessV1 { run_id, pre_tab_id, focused_pane_id, result_tab_id, marker, session_name, incarnation }`; `list-clients` and the independent pane inventory corroborate it. | Each witness reports its own preselected tab, both resolve one identical managed tab and marker, and the concurrent pair leaves exactly one marker candidate. | Any last-key-client target, missing correlation, duplicate candidate, or CLI-only witness rejects the controller/marker representation. Leave all candidates visible; do not choose one or add a name fallback. |
| Durable marker owner and query | From the second client and again from a newly attached client, enumerate all stable tab IDs through the controller, parse each live dump, and cross-check the marker pane against `list-panes`. Detach every client before the fresh attach. | Exactly one query result has the original URL, schema, binding UUID, session nonce, and stable tab ID after detach/reattach. | If configuration is absent, altered, unqueryable, or duplicated, do not advertise `PersistentManagedViewMarkerV1`; return `UnsupportedIdentity` or `DuplicateManagedView`. |
| Session incarnation and replacement | Keep the profile namespace and session name, destroy the session, then create a new session with that same name and attach a fresh client. Compare the old marker tuple with the new inventory before any mutation. | The old session's marker/nonce survives ordinary detach/reattach but is absent or mismatched after replacement; the driver reports `SessionReplaced`. | A same-name replacement that cannot be distinguished rejects the session representation. No focus, create, or automatic rebind is allowed. |
| Stable-ID reuse | Record raw `TabInfo.tab_id` values separately from tab positions, close/reorder tabs, and recreate views. Then replace the same-named session and require an old numeric tab ID to reappear in the new inventory. A bounded in-session churn probe records reuse if seen but never treats non-observation as proof of safety. | When an old numeric ID reappears with an absent or wrong marker/incarnation, the observation is a conflict or replacement, never ownership. | Any path that accepts the reused ID, tab position, or reserved name as ownership rejects the adapter. Preserve the foreign replacement untouched. |
| Create-before-persist window | A test-only barrier fires after native creation and live marker observation but before the portable registry's atomic write. Kill the portable helper, start a fresh client, and inspect native state before retrying. Also record before-create and after-persist controls. | The crash leaves no durable binding record, exactly one native marker candidate, and `RepairRequired(ReattachView)` with zero extra creates. A completed write yields one matching record and marker. | Zero, multiple, or unqueryable candidates after native creation are visible failures; an `Indeterminate` mutation must inspect first and may never blindly create again. |

### Disposable offline-first harness

The later test runs only after the foreground-profile validation passes. It
extends that profile with a second test-owned PTY attachment, a temporary
controller/marker artifact, a fresh `run_id`, private config/data/cache roots,
and pre-granted only the controller's required test-profile permissions:
`ReadApplicationState`, `ChangeApplicationState`, and `OpenFiles`. The grant
must be stored under the test-owned home/cache for the exact local controller
URL and observed as `PermissionRequestResult(true)`; the harness must never
type consent or touch a standing grant.

The external shell harness owns the expected UUID, nonce, foreign tab IDs,
profile-root hashes, and crash barrier. The controller may write witnesses in
the disposable root, but the test accepts a claim only when the independently
queried live Zellij inventory and KDL dump agree. It uses raw PTY keys for the
two client invocations; it does not use a `Run` helper,
`current-tab-info`, a CLI active-tab action, a session-name guess, or a
source-authored layout as its oracle.

The smallest end-to-end invalidation runs first: client A invokes the bound
controller to create one marked tab, client B invokes it fresh, the helper is
killed at the post-create/pre-persist barrier, and client B's fresh inspection
must find exactly one candidate without another create. This sequence jointly
tests the proposed marker carrier, fresh query, cross-client routing, and
crash recovery. It either freezes the candidate or stops the lane before a
product controller exists.

## Acceptance criteria

### Offline

**AC-O1 — A real key from either attached client reaches one recoverable
managed view.** Two individually delivered real-key invocations and one
simultaneous pair leave exactly one live marker candidate. Each witness's
pre-tab equals the distinct foreign tab selected for that PTY, and each result
resolves the same managed tab ID and marker.

Verified by: a new disposable
`tests/zellij-managed-identity-feasibility-test.sh` drives both PTY masters,
parses the controller's JSONL witness, and independently compares
`action list-clients`, `action list-panes --json -a -g -t`, and live dumps.

**AC-O2 — The marker owns a durable, queryable identity rather than a name.**
A fresh controller after full detach/reattach returns exactly one tuple of
canonical controller URL, schema, binding UUID, session nonce, stable tab ID,
and marker-pane ID. A duplicate, absent, or malformed tuple fails closed.

Verified by: the same test starts a fresh client process, enumerates every
live tab through `dump_session_layout_for_tab`, structurally parses its KDL,
and compares the result with the external pane inventory. The expected UUID
and nonce originate in the harness, not in the controller.

**AC-O3 — Session replacement and native ID reuse cannot impersonate the
original view.** Destroying and recreating the same-named session in the same
private namespace, including a reappearing old numeric tab ID, returns
`SessionReplaced` or a typed identity conflict and performs no focus or create
mutation.

Verified by: the test records raw `tab_id` and position before closure and
replacement, requires the old numeric ID in the new inventory, then checks
the driver's typed result and mutation log before it permits an explicit
future `--rebind` path.

**AC-O4 — A native create that outlives its portable caller remains
recoverable without duplication.** Killing the helper after live native
creation and before registry persistence yields one marker candidate and no
registry record. The next operation returns `RepairRequired(ReattachView)`
and issues zero native creates; a duplicate candidate stays a visible failure.

Verified by: a test-only post-create barrier, process kill, fresh-client
inspection, registry-file check, native mutation log, and live inventory
comparison. Before-create and after-persist controls distinguish the three
crash-window states.

**AC-O5 — The spike stays disposable and leaves normal users' Zellij state
alone.** The harness uses temporary profile/config/data/cache roots, removes
them and its named sessions on exit, and leaves the standing config/layout
hashes unchanged.

Verified by: the foreground-profile lifecycle checks bracket every case with
the existing file-state sentinel, session absence, temporary-root removal, and
the exact Zellij 0.44.3 preflight.

### Captain-live

No captain-live acceptance criterion belongs to this feasibility spike. The
two-client key witness is automated through the foreground PTY harness; it is
evidence for a future controller, not a new user-facing keybinding. A captain
drill remains a later Sprint 2 acceptance activity after this native identity
gate accepts a representation.

## Test plan

1. After the foreground-profile gate and shared contract freeze, add the
   smallest two-client test first. It must fail unless the second client
   observes the exact post-create marker after the helper dies before
   persistence; do not substitute a unit fake or a transcript string match.
2. Add the test-only controller and marker pane with the exact local URL,
   run ID, binding UUID, and session nonce. Pre-grant only its temporary
   permissions, wait for a positive permission result, and fail visibly if
   the isolated grant cannot be established.
3. Run sequential A/B key witnesses, then a barrier-synchronized concurrent
   pair. Inspect all tabs through the fresh host query and independently
   compare the live pane inventory, KDL dumps, and client mapping.
4. Detach every client, attach a new client, and repeat the marker query.
   Then replace the same-named session, force old-ID reuse, and require a
   typed no-mutation replacement/conflict result.
5. Exercise the before-create, after-native-create/before-persist, and
   after-persist crash controls. A native timeout or killed helper returns
   `Indeterminate` until inspection supplies a repair path; retries never
   create blindly.
6. Run the focused feasibility test and the foreground-profile lifecycle
   suite. Preserve their global-isolation evidence; a missing dependency,
   permission grant, or native observation is a failed prerequisite, not a
   skipped green result.

## Contract-freeze prerequisites

Do not start the live spike until all of the following are frozen:

- `foreground-attached-client-profile` has passed its PTY foreground-input,
  cleanup, and global-isolation evidence, so both clients can receive real
  keys safely.
- The native CLI and binding-core packets agree on protocol major, one
  request/one result envelope, `Changed`/`Unchanged`/`Indeterminate`, opaque
  session identity, binding UUID format, marker schema, capability names, and
  `inspect`/`repair` failure semantics.
- The profile owns a canonical test-only controller URL, test-only permission
  cache location, crash barrier, and evidence directory. It neither uses nor
  mutates standing Zellij configuration, layouts, cache, data, or grants.

The expected positive decision is narrowly scoped: advertise
`SessionIdentityV1`, `ManagedViewInventoryV1`,
`PersistentManagedViewMarkerV1`, and `CreateMarkedManagedViewV1` for Zellij,
then reconcile with the portable binding contract. Any rejected row returns
the contract to the shared gate. It does not authorize a production controller
or an alternative based on active tabs, panes, cwd, display names, or
best-effort ID matching.

## Documentation change

No user-facing document change is proposed at ideation. The spike changes no
shipping keybinding, layout, or pane behavior. Its implementation must record
the accepted or rejected representation and evidence in the Sprint 1 native
identity gate before any controller documentation claims it.

## Out of scope

Production controller behavior; `Alt Shift z` or `Alt /` product keybindings;
pane adoption; layout toggles; hub, dock, provider, or tmux product code;
global configuration or permission-cache mutation; automatic permission
consent; automatic rebind or repair; a captain-interactive Zellij drill; and
any implementation before the foreground-profile and shared-contract gates.

## Stage Report: ideation

- DONE: Define the exact evidence matrix for multi-client invocation, durable marker ownership/query, session incarnation and replacement, stable-ID reuse, and create-before-persist recovery.
  The matrix specifies real two-PTY key witnesses, a marker-plugin KDL carrier/query, same-name replacement, raw tab-ID reuse, and three crash-window outcomes.
- DONE: Design a disposable, offline-first feasibility harness with explicit prove/reject outcomes and recovery evidence; do not run the spike during ideation.
  The harness is bounded to temporary profile/config/data/cache roots, test-only permissions, external live-state checks, a post-create crash barrier, and fail-closed repair results; no Zellij session was launched here.
- DONE: Bound the later spike from controller, pane-adoption, hub, dock, tmux-product, keybinding, and standing-configuration work, and identify its contract-freeze prerequisites.
  The packet requires the foreground PTY gate plus CLI/binding-envelope freeze, and it expressly withholds production capabilities until the native identity gate accepts them.

### Summary

This packet turns the later spike into a decisive two-client experiment: a
fresh controller must find one persistent marker after a crash, or Zellij
identity remains unsupported. It preserves the user value behind the work—a
single recoverable managed view that never impersonates a foreign tab—without
shipping controller behavior prematurely.
