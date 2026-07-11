---
id: qbf5syzpwvpggp2xgnd5asvf
title: Managed-view binding and shared driver contract
status: ideation
source: managed-view roadmap Sprint 1 binding core; reframed by senior staff review 2026-07-11
started: 2026-07-11T05:21:43Z
completed:
verdict:
score: 0.99
worktree:
issue:
pr:
mod-block:
---

## Problem

Sprint 1 needs a recoverable binding core that can converge on exactly one
managed view for a canonical workspace and native session without trusting a
display name or a reused native ID. The architecture describes that end state,
but the Zellij spike has not yet proved a durable marker, its owner/query path,
or a multi-client invocation witness. The binding core must therefore encode
policy and recovery now while treating Zellij's persisted representation as a
feasibility-gated adapter detail.

## Seed direction

Freeze the portable identity, registry, recovery, result, and fake-adapter
contract together with the native CLI packet seam. Do not freeze a Zellij
socket, tab-marker, controller, layout, pane, or keybinding representation
before `zellij-managed-identity-feasibility` proves it in the approved
disposable profile.

## Dependency boundary

This task is the contract lane in roadmap Sprint 1. It may run in parallel with
the native CLI skeleton ideation, then joins it at the shared contract gate.
The portable registry/fake-adapter lane and the disposable Zellij-feasibility
lane start only after that gate. No controller, live Zellij drill, standing
configuration mutation, pane adoption, hub, dock, provider, or tmux product
work is authorized here.

## Proposed approach

### Durable identity and registry invariants

The portable record is versioned and stored in the platform runtime directory
at `zaphod/bindings-v1.json`. The directory is mode `0700`, the registry is
mode `0600`, and all read-modify-write operations take one registry lock. A
write creates a same-directory temporary file, syncs it, renames it atomically,
then syncs the directory. An interrupted writer therefore leaves a complete
old or new registry, never a partially written JSON document.

```text
BindingV1 {
  binding_id: UUID,                    # generation and desired native marker
  canonical_root: CanonicalRoot,       # absolute, symlink-resolved path
  root_key: sha256(canonical_root),    # filename/index aid, not authority
  mux: MuxKind,
  session: SessionIdentity {
    namespace: OpaqueNativeNamespace,
    native_session_id: OpaqueNativeId,
    incarnation: OpaqueNativeIncarnation,
  },
  managed_view: Option<ManagedViewIdentity> {
    native_view_id: OpaqueNativeId,
    marker: "zaphod.binding.v1/<binding_id>",
    reserved_name: advisory only,
  }
}
```

`CanonicalRoot` is the Git top level when available, otherwise the requested
directory, followed by canonical filesystem resolution. Its path is the
identity; `root_key` is only an index key. `SessionIdentity` is an opaque,
exact triple supplied by a driver: multiplexer namespace, native session ID,
and incarnation. A display name is diagnostic only. The core will not create
or repair a binding unless all three identity fields are supplied with the
driver's advertised identity capability. In particular, this design does not
assume a Zellij socket field, marker store, or tab query protocol.

The registry validates two indexes in one transaction: one active binding per
canonical root, and one canonical root per `(mux, namespace, native ID,
incarnation)`. A duplicate, unknown schema, malformed record, or conflicting
reverse index is a fail-closed `RegistryCorrupt`/`BindingConflict`, never a
best-effort merge. The previous valid file may be retained as an operator
selected recovery input; it is never silently substituted.

### View identity, invalidation, and explicit recovery

Stable IDs locate a native view but do not prove ownership. A view is owned
only when the same exact session identity, stable view ID, and durable marker
all agree. A reserved display name can find a suspicious candidate for an
operator, but cannot validate, focus, or create over it.

| Observation from a fresh native inventory | Binding-core result | Native mutation |
| --- | --- | --- |
| Recorded ID and marker agree exactly once | `Healthy` | focus is permitted |
| Recorded ID missing; exactly one matching marker | `RepairRequired(ReattachView)` | none until explicit repair |
| Recorded ID exists with wrong/missing marker | `ViewIdentityConflict` | none |
| Recorded ID missing; only reserved name exists | `ReservedNameConflict` | none |
| More than one matching marker | `DuplicateManagedView` | none |
| Session namespace, ID, or incarnation differs | `SessionReplaced` | none; require `--rebind` |
| Native inspection cannot establish identity/marker | `UnsupportedIdentity` | none |

The recovery verbs are deliberately distinct:

```text
binding.inspect(root | binding_id) -> BindingInspection       # read-only
binding.unbind(binding_id, explicit_intent) -> Unbound        # registry only
binding.rebind(root, new_session, --rebind) -> Rebound        # atomic index swap
binding.repair(binding_id, action) -> Repaired | RepairRequired
ensure_managed_view(binding_id) -> Created | Focused | Healthy
```

`inspect` never changes the registry or native session. `unbind` removes only
the local association; it never deletes a user-facing native view or session.
`rebind` is an explicit, atomic replacement after both old and new identities
pass preflight and the reverse index is free. `repair(ReattachView)` may update
the recorded stable ID only when an exact session has exactly one matching
marker; `repair(RestoreRegistry)` validates an operator-provided prior record
before atomically installing it. Other states remain visible failures. Normal
`ensure_managed_view` creates only when the binding has no recorded view and
the driver can create a marked view; it never turns a stale ID, name match,
or indeterminate prior mutation into a second create.

This makes the create-before-persist crash window recoverable without guessing:
after a known native create but failed registry write, `inspect` can expose a
single marker candidate and the user can choose `repair`; after an
`Indeterminate` native create, every retry begins with inspection rather than
another mutation.

### Minimal driver-neutral seam and native CLI packet

Sprint 1 freezes only binding operations, not the future pane, dock, provider,
or layout surface. The portable core calls an adapter through these narrow
operations:

```text
inspect_session(candidate) -> SessionObservation
inspect_managed_views(exact_session) -> [ViewObservation]
create_marked_view(exact_session, marker, reserved_name) -> ViewObservation
focus_managed_view(exact_session, native_view_id, marker) -> Focused
```

Every observation returns the native opaque IDs plus the evidence needed to
verify them. Every mutation accepts expected identity/marker values, so an
adapter cannot turn a mismatch into a name-based fallback. Later
`current_view`, `list_panes`, `focus_pane`, `adopt_pane`,
`toggle_managed_layout`, and `open_surface` remain architecture-level
extensions; they are not part of this Sprint 1 interface or implementation.

Capabilities are negotiated rather than inferred from mux kind:
`SessionIdentityV1`, `ManagedViewInventoryV1`, `PersistentManagedViewMarkerV1`,
`CreateMarkedManagedViewV1`, and `FocusManagedViewByStableIdV1`. A driver
without every capability needed for an operation returns `Unsupported` with no
fallback mutation. Zellij may not advertise the marker and identity
capabilities until the native feasibility gate proves them; the fake adapter
does, allowing the portable core to be implemented and tested independently.

The sibling `zaphod-native-cli-skeleton` packet is the required transport seam:

```text
zaphod protocol
  -> Handshake { protocol_version, cli_version, capabilities }

zaphod internal invoke   # one JSON request on stdin, exactly one JSON response
  <- CommandEnvelope { protocol_version, request_id, command }
  -> ResultEnvelope  { protocol_version, request_id, outcome }
```

The handshake's protocol major must match before any command runs; the caller
compares advertised capabilities with the command's required capabilities and
fails before invocation when they are missing. `internal invoke` echoes the
same `request_id`, emits exactly one envelope even for errors, and rejects an
unsupported command without native mutation. The skeleton may initially expose
only inert `health` and `binding.inspect`; this task requires their envelope
behavior, not managed-view behavior or a particular Zellij payload.

### Typed result and mutation model

All portable operations and native envelopes use one tagged result shape:

```text
Outcome<T> =
  | Success { value: T, mutation: Changed | Unchanged }
  | Failure { error: BindingError, mutation: Unchanged | Indeterminate,
              diagnostic: NativeDiagnostic? }
```

`BindingError` is stable and machine-actionable:
`RegistryCorrupt`, `BindingConflict`, `SessionMissing`, `SessionReplaced`,
`UnsupportedIdentity`, `ViewIdentityConflict`, `ReservedNameConflict`,
`DuplicateManagedView`, `RepairRequired`, `Unsupported`, `Busy`,
`PermissionDenied`, `PersistenceFailure`, and `NativeFailure`.

Preflight, parsing, capability, identity, and registry errors are
`Unchanged`. `Indeterminate` is legal only after a native mutation was issued
and completion cannot be observed (timeout, disconnect, or ambiguous native
exit). It forbids a blind repeat: the next action must be `binding.inspect`,
which may yield a repair path or a visible failure. Native stderr, exit code,
and raw payload are diagnostics, never portable error identities.

### Ownership and test boundary

The portable launcher owns canonical-root resolution, registry locking and
atomicity, both uniqueness rules, policy, capability checks, recovery choices,
and user-facing remediation. A native driver owns only truthful observations
and the smallest requested native mutation. The future Zellij controller owns
its proved invocation witness and marker implementation; it must not decide
rebind/repair policy. No tmux product adapter is built in this sprint.

There is no existing product pure function to extend. The prototype's
`src/main.rs::normalize_cwd` is intentionally not reused: it is an exact-text
row-binding helper, while this contract requires filesystem canonicalization
and a reverse-unique session identity. The implementation begins with new,
isolated pure binding-domain functions (canonical-root resolution, registry
invariant validation, inventory classification, and result-state transition)
plus a deterministic fake adapter. This also prevents review findings F1–F10
in the retired foreign-tab retrofit from widening this Sprint 1 lane.

## Riskiest mechanism and live evidence

The riskiest unproven mechanism is Zellij's durable managed-view marker and
fresh-query path across two attached clients. The spike proved that Zellij tab
IDs can be stale or reused, so neither an ID nor a reserved name is adequate;
it did not prove the exact persistent marker owner/query path needed by this
contract.

The first later feasibility check (owned by
`zellij-managed-identity-feasibility`, after
`foreground-attached-client-profile` passes) is deliberately small: in an
isolated Zellij 0.44.3 profile with two attached clients, invoke the proposed
native controller from one client to create a candidate marked view, then use
a fresh invocation from the other client to enumerate and return the exact
same session identity, stable view ID, and marker. Kill the initiating helper
after native creation but before portable persistence, then prove that a
second inspection finds exactly one marker candidate rather than creating a
second view. A missing durable marker/query witness, a mismatched invocation
identity, or an ambiguous inventory rejects this design and sends it back to
the shared contract gate. No live Zellij or tmux drill is run by this task.

## Acceptance criteria

### Offline

**AC-1 — One owned view is the measurable end value.** From a test-harness
inventory with one exact session and no managed view, concurrent
`ensure_managed_view` calls, retries, and a simulated post-create crash leave
exactly one view carrying the binding marker, one root-to-session entry, one
session-to-root entry, and one `create_marked_view` call. A fixture that starts
with two marked views must fail rather than choose one.
Verified by: independently owned, barrier-controlled fake-adapter inventory
and call log plus separately started process tests; expected counts are in the
test harness, not in the registry implementation.

**AC-2 — Registry uniqueness survives concurrency, interruption, and bad
state.** Attempts to bind one canonical root to two different exact sessions,
or one exact session to two roots, produce one durable binding and one typed
conflict. A killed writer leaves a parseable old or new complete registry;
unknown schema, malformed JSON, and a broken reverse index return
`RegistryCorrupt` with `Unchanged`.
Verified by: temporary-runtime process tests that force termination before and
after rename, then parse fixtures authored by the test harness in a new
process.

**AC-3 — Stable-ID invalidation fails closed.** The fresh-inventory table maps
a missing recorded ID plus exactly one marker to `RepairRequired`, a reused ID
with wrong marker to `ViewIdentityConflict`, an unmarked reserved name to
`ReservedNameConflict`, duplicate markers to `DuplicateManagedView`, and any
changed session identity to `SessionReplaced`; all make zero native mutations.
Verified by: table-driven fake inventories that model opaque IDs and markers,
with call counts and expected status tags supplied outside the code under test.

**AC-4 — Inspect, unbind, rebind, and repair are safe and explicit.**
`inspect` performs no registry or native mutation; `unbind` removes only the
local association; `rebind --rebind` atomically preserves reverse uniqueness;
and `repair(ReattachView)` updates a stable ID only for exactly one
same-session marker candidate. Every other repair candidate remains a visible
failure.
Verified by: temporary-registry tests plus fake-adapter call logs that assert
the independently specified mutation count for each command.

**AC-5 — An indeterminate native mutation is never blindly repeated.** A fake
that accepts `create_marked_view` then times out produces `NativeFailure` with
`Indeterminate`; the next `ensure` inspects inventory and never issues a
second create or focus until a safe repair outcome is known.
Verified by: a scripted fake adapter that records operation order and exposes
its inventory only through the next inspection.

**AC-6 — Packet and capability failures are portable and mutation-free.** A
protocol-major mismatch, missing required capability, unsupported command,
or request/response ID mismatch produces one typed `ResultEnvelope` failure
with `Unchanged` and no driver call. A supported request receives exactly one
envelope that preserves its request ID.
Verified by: prebuilt JSON handshake/envelope fixtures and a no-op fake driver;
the assertions parse protocol values rather than matching implementation text.

### Interactive

No interactive acceptance criterion belongs to this contract-only task. It
changes no keybinding, row, layout, or production view. The later isolated
Zellij feasibility task owns the two-client marker drill; the later controller
and pane-adoption tasks own `Alt Shift z`, guarded `Alt /`, and move-pane
demonstrations.

## Test plan

1. **Later, and first before any Zellij implementation:** after the attached
   profile gate, run the bounded two-client disposable Zellij marker/query and
   create-before-persist crash check described above. It is a feasibility-task
   check, not work authorized for this task; failure rejects the provisional
   representation rather than adding a name-based workaround.
2. Build pure tests for canonical-root resolution, `SessionIdentity` equality,
   registry schema validation, reverse-index validation, and classification of
   a supplied view inventory.
3. Run independently started temporary-runtime writers through lock contention,
   root/session conflicts, atomic rename interruption, corrupt-file detection,
   unbind, rebind, and operator-selected registry restoration.
4. Run the deterministic fake adapter through absent, healthy, stale ID,
   unique marker candidate, wrong marker, reserved-name collision, duplicate
   marker, replaced session, capability absence, persistence failure, and
   timeout-after-mutation tables. Assert calls, outcomes, and mutation states,
   not prose emitted by the implementation.
5. Run JSON protocol fixtures for the handshake and one-request/one-response
   CLI envelope, including protocol-major/capability rejection and request-ID
   correlation.
6. Do not run a live Zellij drill, tmux server, controller, or standing
   configuration mutation in this lane.

## Proposed delivery-plan revision

The task has no user-visible document diff: it intentionally changes no
keybinding, row, layout, or behavior. Its planning-only proposal is to amend
`docs/plan-agent-rail.md` → `Target delivery order` as follows:

1. Replace item 1 with: “Freeze and implement the canonical-root/session
   binding registry, full session identity, reverse uniqueness,
   inspect/unbind/rebind/repair commands, typed mutation-aware results, fake
   adapter, and shared convergence suite. Keep native Zellij identity and
   marker representation provisional until its feasibility gate.”
2. Prefix item 2 with: “After the Sprint 1 shared contract and native-identity
   gates pass,” and retain its controller/keybinding scope unchanged.
3. Add a sequencing note that the roadmap’s Sprint 1 gate allows three later
   parallel lanes—native artifact wiring, portable registry/fake suite, and
   disposable Zellij feasibility—but forbids controller work before their
   reconciliation.

This aligns the older product-order prose with `docs/roadmap.md` without
changing the binding-in-plugin, Go grout, or `agent-event` decisions that
remain scoped to the shipped prototype.

## Out of scope

Implementing the registry, native CLI, controller, keybindings, pane adoption,
hub, dock, providers, tmux product adapter, layout-toggle semantics, persistent
item state, automatic rebind, automated permission consent, a global daemon,
foreign-view retrofit, live Zellij drills, and standing multiplexer
configuration changes. The task specifies only the portable contract and the
proofs that later stages must supply.

## Stage Report: ideation

- DONE: Define the exact workspace/session/managed-view binding, driver operations, capability and error model, stable-ID invalidation rules, and portable-versus-native ownership boundary.
  The design specifies a locked atomic registry, session incarnation, binding generation marker, logical operations, adapter primitives, optional capabilities, mutation-aware errors, and fail-closed recovery rules.
- DONE: Name and exercise the riskiest unproven mechanism first, then specify fake-driver and live evidence whose expected values come from outside the implementation.
  The crash-recovery marker was exercised on tmux 3.6a: `$0`, `@1`, and `%0` survived rename/reindex/move; `%0` kept PID `26671`; same-name replacements became `@2` and session `$2`; marker `7df0f1d7-test-generation` identified one recovery candidate.
- DONE: Produce bounded acceptance criteria, test plan, out-of-scope list, and an exact proposed revision to the logical dispatch sequence in docs/plan-agent-rail.md.
  Six offline ACs, the no-interactive-proof rationale, an ordered five-step test plan, explicit exclusions, and replacement text for delivery items 1–2 are recorded above.

### Summary

The shared contract treats native stable IDs as locators, not ownership proof. A portable converger owns policy and validates a session incarnation plus managed-view marker; Zellij and tmux adapters expose narrow observations and mutations. The fake contract suite and registry implementation should land before the Zellij controller.

## Stage Report: ideation (cycle 2)

- DONE: Specify durable canonical-root/session/managed-view identity, marker and stable-ID invalidation, reverse uniqueness, and explicit inspect/unbind/rebind/repair recovery paths.
  `Durable identity and registry invariants` and `View identity, invalidation, and explicit recovery` define opaque full native identity, transactional indexes, a fail-closed inventory table, and all four recovery verbs.
- DONE: Freeze the minimal driver-neutral operations, capabilities, and typed error/result envelope including Unchanged versus Indeterminate, aligned with the proposed native CLI handshake.
  `Minimal driver-neutral seam and native CLI packet` and `Typed result and mutation model` align the contract with `zaphod protocol` and one-request/one-response `zaphod internal invoke` without assuming an unproved Zellij payload.
- DONE: Provide offline-first atomicity/fake-adapter/concurrency evidence and a bounded plan for later native feasibility; exclude controller, pane adoption, hub, dock, provider, tmux product work, and live Zellij drills.
  AC-1–AC-6 and the ordered test plan require process/fake-adapter evidence; the first later two-client feasibility check is explicitly delegated behind the attached-profile gate, while `Out of scope` forbids product and live work here.

### Summary

This refresh replaces the earlier premature native assumptions with an opaque,
capability-gated driver contract. It makes recovery explicit after stale IDs or
indeterminate mutations, gives the native CLI skeleton an interoperable typed
envelope, and keeps the Zellij marker representation provisional until the
separate disposable feasibility lane proves it.
