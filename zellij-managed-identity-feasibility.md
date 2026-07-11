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
sprint: s1-trusted-test-profile-onramp
sprint-lane:
group: contingent-enablement
sprint-readiness: defer
---

## Problem

Production controller work depends on unproved Zellij identity mechanisms: a
ProfileLeaseV1-safe multi-client key invocation witness, a durable marker owner
and structural fresh-query path, a stable session-incarnation representation,
and recovery across create-before-persist crashes and deterministic native ID
reuse.

## Sprint role

Run a disposable, throwaway feasibility spike after the foreground-profile
gate. Prove or reject invocation witness, persistent marker ownership/query,
detach/reattach and replacement identity, deterministic stable-ID reuse, and
crash-window recovery. Do not grow the spike into production controller code.
Its gate either freezes the Zellij representation for integration with the
binding core or returns an explicit unsupported/negative result for revision.

## Proposed approach

### Shared lease and marker-tuple contract

The spike is a secondary consumer of the foreground packet's immutable,
test-only `ProfileLeaseV1` at
`$PROFILE_ROOT/profile-lease-v1.json`; it is not allowed to reconstruct a
profile from a session name, path, active client, or cwd. Before any later
live run, it validates `schema: "zaphod.profile.v1"` and supplies client B the
lease's `attach` inputs verbatim: `zellij_bin`, `config_dir`, `data_dir`,
`cache_dir`, `home_dir`, `namespace`, and `native_session_id`. The base
profile retains ownership of the root, private server, session, primary
client, and final teardown.

This packet creates a distinct test-owned PTY only for client B and records
its OS-observed `{ pid, pgid }` after launch. It alone owns B's process group,
temporary controller artifact, controller-specific disposable permission-cache
entry, evidence directory, and post-create/pre-persist crash barrier. It must
terminate and reap B, remove those B-owned artifacts, and record the release
before `foreground-attached-client-profile` may perform final teardown. It may
not signal the primary client, delete the lease root, alter base-profile
permissions, or tear down the server/session. A missing, malformed, or
non-verbatim lease handoff is a failed prerequisite, not a reason to guess a
replacement profile.

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
  binding_id: canonical lowercase binding UUID,
  session_incarnation: same nonce,
}
MarkerPaneQueryV1 {
  tab_id: observed stable tab locator,
  marker_pane_id: fresh observed plugin-pane ID,
  controller_url: exact canonical test-only local WASM URL,
  schema: "zaphod.binding.v1",
  binding_id: same canonical lowercase UUID,
  session_incarnation: same opaque nonce,
}
```

The carrier is one test-only marker-controller plugin pane in the managed tab.
Its KDL must structurally identify a plugin pane with the exact canonical local
WASM URL and configuration fields `schema: "zaphod.binding.v1"`,
`binding_id: <canonical lowercase UUID>`, and
`session_incarnation: <opaque nonce>`. The fresh query tuple additionally
contains the live `marker_pane_id`; that ID is fresh-query evidence, never
persisted ownership authority. The profile namespace scopes a server; it is
not an incarnation. The tab's display name is advisory only and does not
participate in ownership.

The query path must be a fresh controller invocation, not a CLI action guessed
from the active client. The controller receives the actual temporary
`MessagePlugin` keybinding, obtains its host-resolved focused pane/tab, gets
the stable tab IDs from `TabUpdate`, and calls
`dump_session_layout_for_tab(tab_id)` for every candidate. It structurally
parses the live KDL plugin node and its configuration—never a text or regex
match—and accepts only one node with the exact URL, schema, UUID, and nonce.
It then cross-checks fresh `action list-panes --json -a -g -t` output for the
same tab, plugin-pane ID, and URL before returning `MarkerPaneQueryV1`.
`list-panes` cannot substitute for the KDL query because its `PaneInfo`
exposes a plugin URL but not plugin configuration. Zero, malformed,
mismatched, or multiple tuples are fail-closed and may not focus or create by
name.

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
| Exact multi-client invocation | Read one fresh `ProfileLeaseV1`; start B with its `attach` object verbatim in a distinct test-owned PTY and record B's OS-observed PID/PGID. Give A and B distinct, known foreign tabs. Deliver the real temporary `Alt Shift z` binding through each PTY, then concurrently through both. The controller writes a test-owned `InvocationWitnessV1 { run_id, client_pid, client_pgid, pre_tab_id, focused_pane_id, result_tab_id, marker, session_name, incarnation }`; `list-clients` and the independent pane inventory corroborate it. | Each witness reports its own preselected tab, both resolve one identical managed tab and marker, and the concurrent pair leaves exactly one marker candidate. | Any altered lease input, last-key-client target, missing correlation, duplicate candidate, or CLI-only witness rejects the representation. Leave all candidates visible; do not choose one or add a name fallback. |
| Durable marker owner and query | From B and again from a newly attached client, enumerate all stable tab IDs through the controller, structurally parse every live KDL dump, and cross-check each candidate against `list-panes`. Detach every client before the fresh attach. | Exactly one `MarkerPaneQueryV1` has the original canonical URL, `schema`, lowercase binding UUID, session-incarnation nonce, stable tab ID, and live marker-pane ID after detach/reattach. | If configuration is absent, altered, unqueryable, or duplicated, return canonical `Unsupported` with `mutation: Unchanged`; Zellij advertises no marker or identity capability. |
| Session incarnation and replacement | Keep the profile namespace and session name, destroy the session, then create a new session with that same name and attach a fresh client. Compare the old marker tuple with the new inventory before any mutation. | The old session's marker/nonce survives ordinary detach/reattach but is absent or mismatched after replacement; the driver reports `SessionReplaced`. | A same-name replacement that cannot be distinguished rejects the session representation. No focus, create, or automatic rebind is allowed. |
| Stable-ID reuse | Run a harness-controlled, repeatable native sequence that force/observes an old raw `TabInfo.tab_id` reappear after close/reorder/recreate and same-name session replacement; record the old and fresh inventories before any mutation. A bounded churn probe that merely fails to see reuse is not a witness. | When the deterministically reappearing old numeric ID has an absent or wrong marker/incarnation, the observation is a conflict or replacement, never ownership. | If the harness cannot force and observe reuse deterministically, record an explicit negative `Unsupported` result with `mutation: Unchanged`; Zellij advertises none of `driver.session-identity.v1`, `driver.managed-view-inventory.v1`, `driver.persistent-managed-view-marker.v1`, or `driver.create-marked-managed-view.v1`. Any path that accepts the reused ID, tab position, or reserved name as ownership rejects the adapter. |
| Create-before-persist window | B's test-owned crash barrier fires after native creation and live marker observation but before the portable registry's atomic write. Kill the portable helper, start a fresh client, and inspect native state before retrying. Also record before-create and after-persist controls; B releases before profile final teardown. | The crash leaves no durable binding record, exactly one native marker candidate, and `RepairRequired(ReattachView)` with zero extra creates. A completed write yields one matching record and marker. | Zero, multiple, or unqueryable candidates after native creation are visible failures; an `Indeterminate` mutation must inspect first and may never blindly create again. |

### Disposable offline-first harness

The later test runs only after the foreground-profile validation passes and
prints a fresh immutable `ProfileLeaseV1` path. The harness consumes that one
lease; it does not allocate a second profile. It extends the leased profile
with B's separate PTY, temporary controller/marker artifact, fresh `run_id`,
and controller-specific pre-granted permissions:
`ReadApplicationState`, `ChangeApplicationState`, and `OpenFiles`. The grant
is a test-owned cache entry for the exact canonical controller URL within the
disposable lease paths and must be observed as `PermissionRequestResult(true)`;
the harness never types consent, touches a standing grant, or claims ownership
of the profile's cache root.

The external shell harness owns the expected UUID, nonce, foreign tab IDs,
profile-root hashes, B PID/PGID observation, B release receipt, and crash
barrier. The controller may write witnesses only in the disposable evidence
directory, but the test accepts a claim only when the independently queried
live Zellij inventory and structural KDL query agree. It uses raw PTY keys for
the two client invocations; it does not use a `Run` helper,
`current-tab-info`, a CLI active-tab action, a session-name guess, or a
source-authored layout as its oracle.

The smallest end-to-end invalidation runs first: A invokes the bound
controller to create one marked tab, B—launched only from verbatim lease
inputs—invokes it fresh, the B-owned barrier kills the helper at
post-create/pre-persist, and B's fresh inspection must find exactly one
`MarkerPaneQueryV1` candidate without another create. This sequence jointly
tests the proposed marker carrier, structural fresh query, cross-client
routing, and crash recovery. It either freezes the candidate or stops the
lane before a product controller exists; B is released before foreground final
teardown in every outcome.

## Acceptance criteria

### Offline

**AC-O1 — The secondary client uses the foreground lease without taking over
the profile.** A run starts B only from one fresh valid `ProfileLeaseV1`, with
all `attach` values passed verbatim; it records B's OS-observed PID/PGID and
does not infer a namespace, session, or path. B's process group, temporary
controller, permission-cache entry, evidence directory, and crash barrier are
released before the foreground profile's final teardown.

Verified by: a disposable
`tests/zellij-managed-identity-feasibility-test.sh` records the lease input,
B PID/PGID, B-owned artifact paths, B reap result, and the foreground
lifecycle's later teardown receipt; the profile's root and primary-client
ownership remain independently recorded by its existing lifecycle suite.

**AC-O2 — A real key from either attached client reaches one recoverable
managed view.** Two individually delivered real-key invocations and one
simultaneous pair leave exactly one live marker candidate. Each witness's
pre-tab equals the distinct foreign tab selected for that PTY, and each result
resolves the same managed tab ID and marker.

Verified by: the feasibility test drives both PTY masters, parses the
controller's JSONL witness, and independently compares `action list-clients`,
`action list-panes --json -a -g -t`, and fresh controller KDL queries.

**AC-O3 — The marker owns a durable, queryable identity rather than a name.**
A fresh controller after full detach/reattach returns exactly one
`MarkerPaneQueryV1` tuple of canonical controller URL, `schema`, canonical
lowercase binding UUID, session-incarnation nonce, stable tab ID, and live
marker-pane ID. A duplicate, absent, malformed, or mismatched tuple is the
canonical `Unsupported` result with `mutation: Unchanged` and no Zellij
identity capability advertisement.

Verified by: the test starts a fresh client process, enumerates every live tab
through `dump_session_layout_for_tab`, structurally parses each plugin node and
its configuration, then joins the candidate to the external pane inventory by
tab, pane ID, and URL. The expected UUID and nonce originate in the harness,
not in the controller.

**AC-O4 — Session replacement and deterministic native ID reuse cannot
impersonate the original view.** The harness must force and observe a previous
raw numeric `tab_id` reappear after a controlled reuse sequence and
same-named-session replacement. A wrong or absent marker/incarnation produces
a no-mutation replacement/conflict result; if native reuse cannot be forced
and observed deterministically, the gate records an explicit negative
`Unsupported` result and Zellij advertises none of
`driver.session-identity.v1`, `driver.managed-view-inventory.v1`,
`driver.persistent-managed-view-marker.v1`, or
`driver.create-marked-managed-view.v1`.

Verified by: the harness records old and new raw inventories plus its
repeatable reuse control before any focus/create operation, then checks the
correlated result and mutation log. Mere non-observation in a bounded churn
loop is an unsupported outcome, never a passing safety result.

**AC-O5 — A native create that outlives its portable caller remains
recoverable without duplication.** Killing the helper after live native
creation and before registry persistence yields one marker candidate and no
registry record. The next operation returns `RepairRequired(ReattachView)`
and issues zero native creates; a duplicate candidate stays a visible failure.

Verified by: B's test-only post-create barrier, process kill, fresh-client
inspection, registry-file check, native mutation log, and live inventory
comparison. Before-create and after-persist controls distinguish the three
crash-window states.

**AC-O6 — The spike stays disposable and leaves normal users' Zellij state
alone.** The harness uses the foreground profile's temporary roots without
claiming their ownership; it releases B and its artifacts first, then lets the
foreground lifecycle remove its named session and roots. Standing
configuration and layout hashes remain unchanged.

Verified by: the foreground-profile lifecycle checks bracket every case with
the existing file-state sentinel, B-release receipt before final teardown,
session absence, temporary-root removal, and the exact Zellij 0.44.3 preflight.

### Captain-live

No captain-live acceptance criterion belongs to this feasibility spike. The
two-client key witness is automated through the foreground PTY harness; it is
evidence for a future controller, not a new user-facing keybinding. A captain
drill remains a later Sprint 2 acceptance activity after this native identity
gate accepts a representation.

## Test plan

1. After the foreground-profile gate and shared contract freeze, validate the
   newly emitted `ProfileLeaseV1` before allocating B. Pass the complete
   `attach` object verbatim, record B's OS-observed PID/PGID, and stop as a
   failed prerequisite if the lease, B ownership boundary, or isolated
   controller permission entry cannot be established.
2. Add the test-only controller and marker pane with the exact canonical local
   URL, `schema: "zaphod.binding.v1"`, canonical lowercase binding UUID, and
   session-incarnation nonce. Pre-grant only its temporary permissions, wait
   for a positive permission result, and fail visibly if the isolated grant
   cannot be established.
3. Run the smallest two-client invalidation first: A creates one marked tab,
   B invokes fresh, the B-owned barrier kills the helper before persistence,
   and B's structural fresh query must find exactly one `MarkerPaneQueryV1`
   without another create. Do not substitute a unit fake or transcript match.
4. Run sequential A/B key witnesses, then a barrier-synchronized concurrent
   pair. Detach every client, attach a fresh client, enumerate every tab via
   `dump_session_layout_for_tab`, structurally parse KDL, and independently
   join the one candidate with live pane ID and URL inventory evidence.
5. Exercise the harness-controlled deterministic reuse sequence before any
   capability decision. It must force and observe an old numeric tab ID in a
   fresh inventory; otherwise produce the explicit negative `Unsupported`
   result and advertise no Zellij `driver.*` identity token. Then replace the
   same-named session and require a typed no-mutation replacement/conflict
   result.
6. Exercise the before-create, after-native-create/before-persist, and
   after-persist crash controls. A native timeout or killed helper returns
   `Indeterminate` until inspection supplies a repair path; retries never
   create blindly. Reap B and remove only B-owned artifacts before the
   foreground profile performs final cleanup.
7. Run the focused feasibility test and the foreground-profile lifecycle
   suite. Preserve their global-isolation evidence; a missing dependency,
   permission grant, deterministic reuse witness, or native observation is a
   failed/negative prerequisite, not a skipped green result.

## Contract-freeze prerequisites

Do not start the live spike until all of the following are frozen:

- `foreground-attached-client-profile` has passed its PTY foreground-input,
  cleanup, and global-isolation evidence; it publishes one immutable
  `ProfileLeaseV1` only after A is ready and remains the sole owner of the
  base session, primary client, root, and final teardown. The feasibility lane
  has a tested verbatim attach and B-release handoff.
- The native CLI and binding-core packets agree on protocol major 1, one
  request/one result envelope, the exact lower-case `driver.*.v1` capability
  tokens, and `Changed`/`Unchanged`/`Indeterminate` mutation semantics. Their
  failure handling uses the canonical `MalformedEnvelope`, `ProtocolMismatch`,
  `CorrelationMismatch`, and `Unsupported` codes; only an issued native
  mutation whose completion cannot be observed may be `Indeterminate`, and it
  requires fresh inspection before any retry.
- The feasibility lane owns the canonical test-only controller URL, B's
  controller-specific permission-cache entry, B's crash barrier, and its
  evidence directory within the disposable lease paths. It neither uses nor
  mutates standing Zellij configuration, layouts, cache, data, or grants, and
  it leaves base-profile root/session teardown to the foreground packet.

The expected positive decision is narrowly scoped: only after every evidence
row—including a deterministic native-ID-reuse witness—passes may Zellij
advertise `driver.session-identity.v1`, `driver.managed-view-inventory.v1`,
`driver.persistent-managed-view-marker.v1`, and
`driver.create-marked-managed-view.v1`. `SessionIdentityV1`,
`ManagedViewInventoryV1`, `PersistentManagedViewMarkerV1`, and
`CreateMarkedManagedViewV1` remain conceptual representations for those
tokens; `driver.focus-managed-view-by-stable-id.v1` stays absent. Any failed
row, and specifically an unavailable deterministic reuse witness, returns a
correlated negative `Unsupported` decision with `mutation: Unchanged` and no
Zellij identity capability. This does not authorize a production controller or
an alternative based on active tabs, panes, cwd, display names, or best-effort
ID matching.

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

## Stage Report: ideation (cycle 2)

- DONE: Consume ProfileLeaseV1 for the secondary PTY client and assign its process-group, controller, crash-barrier, and teardown evidence without touching base-profile ownership.
  The revised lease contract requires verbatim B attach inputs, OS-observed PID/PGID, B-owned artifacts, and B release before foreground final teardown.
- DONE: Adopt the binding-ID/incarnation marker-pane configuration and fresh structural query mapping as the native identity proof target.
  `MarkerPaneQueryV1` now fixes the URL/schema/UUID/nonce/pane-ID tuple and requires structural KDL plus fresh pane-inventory cross-checking.
- DONE: Make deterministic native-ID reuse a mandatory witness or an explicit unsupported result; do not treat non-observation as success or run the spike during ideation.
  The evidence matrix, ACs, and plan now withhold every Zellij identity capability on an unavailable reuse witness; this was a design-only state-record update.

### Summary

This alignment rework makes the feasibility lane a well-bounded consumer of
the foreground lease and a strict producer of either native identity evidence
or an explicit negative result. No spike, keybinding, controller, or standing
Zellij configuration was run or changed during this ideation stage.

## Stage Report: ideation (cycle 3)

- DONE: Repair AC-O3 evidence or explicitly defer it to implementation with a cited reason.
  AC-O3 evidence: product HEAD `d9226f2` pins `zellij-tile`/`zellij-utils` 0.44.3 (`Cargo.lock:2735-2752`); `src/main.rs:466-482` maps `TabUpdate.tab_id`, and `src/main.rs:905-924` requests a current `dump_session_layout_for_tab` result and treats that server dump as authoritative. `SPEC.md:73-83,280-286` and `docs/docking-approach.md:777-803` independently establish URL-plus-configuration instance identity, stable-ID routing, and live dump/layout state as the identity oracle.
  Those sources support the proposed query's available inputs, but not AC-O3's end value: no per-binding marker, full-detach fresh client, structural KDL parser, or tuple result exists yet. That proof is deliberately deferred to implementation, where the test-only controller and leased live harness can exercise it; no live Zellij drill ran in ideation.
- DONE: Preserve the fail-closed native-identity decision and no-drill boundary.
  AC-O4 deferred proof remains mandatory: implementation must force and observe native raw-ID reuse, or return correlated `Unsupported` with `mutation: Unchanged` and advertise no Zellij identity capability. A tab ID, position, session name, display name, active client, or cwd is never an ownership fallback; neither a drill nor identity fallback was introduced.
- DONE: Append a report-only ideation cycle; do not edit product files or redesign scope.
  Sources inspected exactly: `Cargo.lock:2735-2752`, `src/main.rs:466-482,905-924`, `SPEC.md:73-83,181-184,280-286`, `docs/docking-approach.md:469-480,523-529,777-803`, and `docs/zaphod-workspace-architecture.md:281-294,437-441` at product HEAD `d9226f2`; only this split-root state record changed.

### Summary

The native query design now has auditable, independent source support for its
live server inputs and URL/configuration carrier, while its durable-marker end
value remains honestly unproved. Implementation must either demonstrate the
fresh structural tuple in the leased harness or retain the correlated negative
result; this report authorizes neither a live drill nor a fallback identity.

### AC-by-AC evidence / deferred proof

- AC-O1 — deferred to implementation: verbatim lease use, B PID/PGID, and release receipts require the foreground artifact plus a real B process; no run occurred here.
- AC-O2 — deferred to implementation: two PTY key witnesses and one-marker convergence require the leased live controller/harness.
- AC-O3 — source-supported design, implementation proof deferred: the cited pinned API, stable-ID route, and server URL/configuration oracle support the query inputs; only a fresh post-detach structural tuple can satisfy the AC.
- AC-O4 — deferred and fail-closed: no deterministic native-ID-reuse witness exists yet; its only acceptable alternative is the correlated `Unsupported`/`Unchanged` negative result with no `driver.*` identity capability.
- AC-O5 — deferred to implementation: the post-create/pre-persist barrier, fresh inspection, and zero-extra-create check require a live native mutation boundary.
- AC-O6 — deferred to implementation: temporary-root/hash and B-release evidence require the foreground lifecycle suite around the leased harness.
