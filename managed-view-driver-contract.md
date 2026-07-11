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
sprint: s1-trusted-test-profile-onramp
sprint-lane:
group: contingent-enablement
sprint-readiness: defer
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

`binding.ensure(canonical_root, exact_session)` is the initial binding verb.
Under the same lock it returns the existing record only when both indexes name
the same binding; it otherwise creates both index entries atomically or returns
`BindingConflict`. It performs no native view mutation. Later public
`ensure_workspace` may compose session creation/adoption with this verb, but
that native behavior is outside this Sprint 1 core.

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
binding.ensure(canonical_root, exact_session) -> Bound | Existing
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

Capabilities are negotiated rather than inferred from mux kind. The exact v1
wire tokens, command requirements, and error behavior are frozen in the
shared-contract addendum below; the CamelCase names used here are type names,
not alternate advertised capability strings. A driver without every capability
needed for an operation returns `Unsupported` with no fallback mutation. Zellij
may not advertise its identity or marker capabilities until the native
feasibility gate proves them; the fake adapter may advertise them in portable
tests.

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
  | Failure { error: ErrorCodeV1, mutation: Unchanged | Indeterminate,
              diagnostic: NativeDiagnostic? }
```

`ErrorCodeV1` includes the canonical protocol errors in the addendum and these
stable, machine-actionable binding errors:
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

## Shared-contract addendum — Sprint 1 gate

This is the concrete reframing required by the staff review's “Cross-packet
interface and ownership assessment” and “Actionable staff recommendation.” It
implements roadmap step 2's contract freeze before the three Sprint 1 lanes
begin; it neither authorizes a Zellij controller nor advertises an unproved
Zellij identity feature. Sources: `docs/roadmap.md` — “Dispatch and merge
order”; `sprint-1-staff-coherence-review.md` — “Cross-packet interface and
ownership assessment.”

### Go binding-core owner and command dispatch

`grout/internal/bindingcore` is the canonical Go owner of `BindingV1`, registry
locking and atomic writes, canonical-root resolution, reverse uniqueness,
inventory classification, recovery policy, and the injected `Driver` interface.
It exposes a `Service`; `grout/internal/zaphodcli` decodes the typed envelope
and dispatches once to that service; `grout/cmd/zaphod` only owns process
startup, stdin/stdout, exit classification, and the handshake. Neither shell
package may open the registry, infer an identity from a name or active client,
choose a repair, or retry an indeterminate mutation. This uses the existing
`grout` module while preserving the CLI packet's required command/wire
boundary. Sources: `grout/go.mod`; `zaphod-native-cli-skeleton.md` —
“Ownership and artifact boundary”; `docs/roadmap.md` — “Delivery rules.”

| Exact command discriminant | Binding-core destination | Required v1 capability | Boundary |
| --- | --- | --- | --- |
| `binding.ensure` | `Service.Ensure` | `binding.ensure.v1` and `driver.session-identity.v1` when the service resolves a session candidate | Registry create/reuse only; no native-view mutation. |
| `binding.inspect` | `Service.Inspect` | `binding.inspect.v1`; native classification additionally needs the driver capabilities named below | Read-only; the skeleton returns `Unsupported` until this service is linked. |
| `binding.unbind` | `Service.Unbind` | `binding.unbind.v1` | Removes only the local binding. |
| `binding.rebind` | `Service.Rebind` | `binding.rebind.v1` and `driver.session-identity.v1` | Replaces the exact session only after reverse-index preflight. |
| `binding.repair` | `Service.Repair` | `binding.repair.v1`; `ReattachView` additionally needs inventory and marker capabilities | The service, never the CLI, selects the fail-closed repair transition. |
| `binding.ensure-managed-view` | `Service.EnsureManagedView` | `binding.ensure-managed-view.v1` plus all required driver capabilities | Reserved and unadvertised until native identity evidence accepts it. |

The `zaphod-native-cli-skeleton` packet must route the first five discriminants
to this service when implemented, and must not acquire registry policy. The
`zellij-managed-identity-feasibility` packet consumes only the injected driver
interface and remains unable to rebind, repair, or write registry state.

### Protocol-v1 capability, error, and mutation matrix

All `Handshake`, `CommandEnvelope`, and `ResultEnvelope` values use
`protocol_version: 1`. A handshake advertises only capabilities actually
implemented for its selected driver. The initial skeleton therefore advertises
only `protocol.handshake.v1`, `protocol.invoke.v1`, and `health.v1`; it
advertises no `binding.*` or `driver.*` token. These exact strings replace any
CamelCase capability spelling on the wire.

| Layer | Exact v1 capability token | Advertise when | Required by |
| --- | --- | --- | --- |
| Transport | `protocol.handshake.v1` | The binary can emit one handshake line. | `zaphod protocol` |
| Transport | `protocol.invoke.v1` | The binary can read one envelope and emit one correlated result. | `zaphod internal invoke` |
| Transport | `health.v1` | Side-effect-free health is implemented. | `health` |
| Binding | `binding.ensure.v1`, `binding.inspect.v1`, `binding.unbind.v1`, `binding.rebind.v1`, `binding.repair.v1` | The corresponding `bindingcore.Service` method is integrated and process-tested. | The same-named command only |
| Binding | `binding.ensure-managed-view.v1` | The native convergence method and all of its driver preconditions are accepted. | `binding.ensure-managed-view` only |
| Driver | `driver.session-identity.v1` | A driver returns an exact namespace, native session ID, and incarnation. | Ensure/rebind and native inspection |
| Driver | `driver.managed-view-inventory.v1` | A driver returns a fresh exact-session view inventory. | Inspect, repair, and native convergence |
| Driver | `driver.persistent-managed-view-marker.v1` | A driver can persist and freshly query the exact marker tuple below. | Inspect, repair, and native convergence |
| Driver | `driver.create-marked-managed-view.v1` | A driver can create a view with that marker and confirm the result. | Native convergence |
| Driver | `driver.focus-managed-view-by-stable-id.v1` | A driver proves focus by stable ID plus marker. | Native convergence |

`SessionIdentityV1`, `ManagedViewInventoryV1`,
`PersistentManagedViewMarkerV1`, and `CreateMarkedManagedViewV1` in the
feasibility packet are the conceptual types for the four corresponding
`driver.*` tokens above. A fake driver may advertise those tokens during the
portable suite. Zellij advertises none of them until the feasibility spike
passes its two-client, persistence/query, replacement, deterministic-ID-reuse,
and crash-window evidence. Non-observation of ID reuse is not proof: the spike
must provide a deterministic reuse witness or report an explicit unsupported
result. `driver.focus-managed-view-by-stable-id.v1` remains absent for Zellij
until a later focus proof; it is not implied by a successful marker spike.

| Boundary condition | Canonical `ErrorCodeV1` | Required result and effect |
| --- | --- | --- |
| Invalid JSON or a missing/wrong required envelope field | `MalformedEnvelope` | Emit one v1 `ResultEnvelope` with the decoded nonempty `request_id`, or `""` when it cannot be decoded; `mutation: Unchanged`; do not dispatch. |
| Request or handshake protocol major is not 1 | `ProtocolMismatch` | Emit or normalize one v1 failure under the decoded/sent request ID; `mutation: Unchanged`; do not dispatch. |
| A caller receives a parseable response whose ID differs from its sent ID | `CorrelationMismatch` | Normalize locally to one failure under the sent ID; `mutation: Unchanged`; ignore the response value and do not retry or call a driver. |
| Unknown command or missing advertised requirement | `Unsupported` | Emit one correlated failure with `mutation: Unchanged`; do not dispatch or fall back. |

`Success` uses `Changed` only after a mutation is observed as complete and
durable; it uses `Unchanged` for a read, reuse, or no-op. A failure is
`Unchanged` unless a native mutation was issued and its completion cannot be
observed; only then it is `Indeterminate`. An `Indeterminate` result requires
fresh `binding.inspect` before any retry and never permits a name-based second
create. The CLI packet must implement this matrix in its envelope fixtures;
the binding-core packet must use it for every service transition; the
feasibility packet must leave Zellij tokens absent on any failed row.

### Disposable-profile lease and marker-pane handoffs

`foreground-attached-client-profile` must publish one immutable, test-only
`ProfileLeaseV1` at `$PROFILE_ROOT/profile-lease-v1.json` and print its path
only after the foreground client is ready. It is an interface, not a standing
configuration file:

```text
ProfileLeaseV1 {
  schema: "zaphod.profile.v1",
  profile_root: absolute disposable root,
  namespace: opaque exact private Zellij server namespace,
  native_session_id: exact disposable session name,
  primary_client: { pid, pgid },        # OS-observed foreground PGID
  attach: { zellij_bin, config_dir, data_dir, cache_dir, home_dir,
            namespace, native_session_id },
  teardown_owner: "foreground-attached-client-profile"
}
```

`namespace` and `native_session_id` must be passed back verbatim, never
derived from a path, display name, active client, or cwd. This lease does not
claim an incarnation: `zellij-managed-identity-feasibility` owns the separate
nonce proof before a `SessionIdentityV1` may be advertised. To attach client B,
that packet supplies the immutable `attach` inputs plus a distinct test-owned
PTY and returns an OS-observed `{ pid, pgid }` for B; it owns B's process group,
temporary controller, permission cache, evidence, and crash barrier. The
profile packet alone owns the base session, primary client, root, and final
teardown. The CLI packet owns only `$PROFILE_ROOT/bin/zaphod`; it creates no
client and may not tear down the lease. Each lane uses a fresh lease for its
own test run; a later integrated run serializes use of one lease and releases
the secondary client before the profile performs final cleanup. Sources:
`foreground-attached-client-profile.md` — “Proposed approach”;
`zaphod-native-cli-skeleton.md` — “Ownership and artifact boundary”; and
`zellij-managed-identity-feasibility.md` — “Disposable offline-first harness.”

The portable logical marker and the provisional Zellij marker pane map exactly
as follows once the spike is allowed to test it:

| Binding-core value | Required marker-pane evidence | Fresh query rule |
| --- | --- | --- |
| `BindingV1.binding_id` | Canonical lowercase UUID in plugin configuration `binding_id`; logical marker string is `zaphod.binding.v1/<binding_id>`. | Query matches the UUID exactly, never a reserved tab name. |
| `BindingV1.session.incarnation` | Plugin configuration `session_incarnation` contains the exact opaque nonce. | Query rejects a missing or different nonce as replacement/conflict. |
| Marker schema | Plugin configuration `schema == "zaphod.binding.v1"`. | Query rejects missing, altered, or duplicate schema tuples. |
| Driver-supplied controller URL | The marker pane has the exact test-owned canonical local WASM URL. | Enumerate every tab through `dump_session_layout_for_tab(tab_id)`, structurally parse KDL, then cross-check the same plugin pane ID and URL with `list-panes --json -a -g -t`. |

The fresh controller query returns `{ tab_id, marker_pane_id, controller_url,
schema, binding_id, session_incarnation }`; `tab_id` becomes the recorded
native-view locator only after the exact tuple and session match. The
`marker_pane_id` is fresh-query evidence, not persisted ownership authority.
The query runs through a fresh controller invocation, never a CLI active-tab
action or a guessed client. Exactly one matching tuple can support the existing
healthy or reattach paths;
zero, malformed, mismatched, or multiple tuples take the existing fail-closed
unsupported/conflict paths and never focus or create by name. The feasibility
packet must consume this configuration/query mapping and record its proof or
negative result; the CLI packet must carry it only through typed commands; the
profile packet must supply the lease fields without claiming native identity.

## Riskiest mechanism and live evidence

The riskiest unproven mechanism is Zellij's durable managed-view marker and
fresh-query path across two attached clients. Existing evidence makes a stale
or reused tab ID unsafe, but it has not supplied the deterministic native
ID-reuse witness, persistent marker owner, or query path needed by this
contract. Neither an ID nor a reserved name is therefore adequate.

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
`binding.ensure` and `ensure_managed_view` calls, retries, and a simulated
post-create crash leave exactly one view carrying the binding marker, one
root-to-session entry, one session-to-root entry, and one
`create_marked_view` call. A fixture that starts with two marked views must
fail rather than choose one.
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
`binding.ensure` atomically creates or reuses only an exact root/session pair;
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
malformed envelope returns `MalformedEnvelope`, a protocol-major mismatch
returns `ProtocolMismatch`, a response-ID mismatch normalizes to
`CorrelationMismatch`, and a missing required capability or unsupported command
returns `Unsupported`. Every failure is one typed `ResultEnvelope` with
`Unchanged` and no driver call; a supported request receives exactly one
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
   CLI envelope, including malformed-envelope, protocol-major, capability, and
   correlation rejection with the frozen `ErrorCodeV1` and mutation outcomes.
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

## Stage Report: ideation (cycle 3)

- DONE: Name the canonical Go binding-core owner and map each binding command to it without assigning registry policy to the CLI shell.
  The shared-contract addendum assigns `grout/internal/bindingcore` and maps all six discriminants to `Service` methods while limiting `cmd/zaphod` and `internal/zaphodcli` to process and wire boundaries.
- DONE: Freeze one v1 handshake/capability/error matrix, including correlation and mutation outcomes, with Zellij identity capability absent until native proof.
  The matrix fixes transport, binding, and driver tokens; canonical `MalformedEnvelope`, `ProtocolMismatch`, and `CorrelationMismatch`; and the only legal `Changed`, `Unchanged`, and `Indeterminate` transitions.
- DONE: Define the shared profile-test interface and binding-ID-to-marker mapping needed by the CLI and feasibility lanes, with explicit dependency handoffs.
  `ProfileLeaseV1` assigns the profile, CLI, and second-client teardown boundaries; the marker table binds the UUID and incarnation to KDL configuration plus a fresh structural query.

### Summary

The addendum converts the staff review's three implicit seams into a single,
testable Sprint 1 contract. It reserves native Zellij behavior until the
disposable feasibility lane supplies positive evidence or an explicit negative
result, while allowing the portable binding and native-CLI lanes to proceed on
one auditable interface.

## Stage Report: ideation (cycle 4)

- DONE: Repair AC-1 evidence or explicitly defer it to implementation with a cited reason.
  AC-1's duplicate-free end value is supported by the independent Sprint 1 contract gate and exit requirement in `docs/roadmap.md:68-86` and the senior review's “Required gates and sprint exit” in `docs/agent-rail-dev/.spacedock-state/managed-view-sprint-review.md:66-75`; it is not runtime proof. The still-unmet barrier-controlled fake inventory/call log and separately started process test belong to implementation because only the built binding core can expose the create count, lock behavior, and resulting registry state for later independent rerun.
- DONE: Repair AC-3 through AC-6 evidence or explicitly defer each to implementation with cited reasons.
  AC-3: `docs/roadmap.md:11-14,82-85` requires stable IDs to be locators rather than ownership proof and recovery without display-name trust; `SPEC.md:179-187` independently records that manifest state, not sticky flags, is authoritative. The table-driven fake-inventory/call-count proof is still implementation work, while native reuse and marker proof remain explicitly owned by `docs/agent-rail-dev/.spacedock-state/zellij-managed-identity-feasibility.md:126-134`.
  AC-4: `docs/roadmap.md:36-39,68-70,82-83` independently requires atomic storage, reverse uniqueness, explicit repair, and inspect/reconcile/remove outcomes. Temporary-registry and mutation-log proof is still implementation work because it must exercise the actual atomic write and command boundaries, then validation reruns it externally.
  AC-5: the senior review requires typed mutation results and says indeterminate operations reconcile before retry (`docs/agent-rail-dev/.spacedock-state/managed-view-sprint-review.md:66-75`); its P1 finding rejects a coarse timeout result (`:40-42`). The scripted fake operation-order proof is still implementation work because it must observe a real service inspect before a second create, not merely restate the matrix.
  AC-6: `docs/roadmap.md:11,33-35,49-51,74-75` requires transport before behavior and a versioned typed interface, while the senior review identifies the missing stable error semantics (`docs/agent-rail-dev/.spacedock-state/managed-view-sprint-review.md:39-44`). JSON envelope/no-op-driver fixtures remain implementation work because one-result correlation and zero driver calls are executable process behavior; the later validator must black-box rerun them.
- DONE: Append a report-only ideation cycle; do not edit product files or redesign scope.
  Preserved the bindingcore owner, protocol-v1 matrix, immutable `ProfileLeaseV1` handoff, and fail-closed identity direction; no Zellij drill or runtime pass is claimed. Sources inspected: this record §§ “Shared-contract addendum,” “Acceptance criteria,” and “Test plan”; `docs/roadmap.md:9-86`; `docs/agent-rail-dev/.spacedock-state/managed-view-sprint-review.md:30-75`; `SPEC.md:179-208`; `docs/docking-approach.md:775-805`; `docs/agent-rail-dev/README.md:100-139`; `docs/agent-rail-dev/.spacedock-state/zellij-managed-identity-feasibility.md:35-162`; `docs/agent-rail-dev/.spacedock-state/zaphod-native-cli-skeleton.md:77-120`; `docs/plan-agent-rail.md:10-35`; and `docs/review-findings-2026-07-07.md:1-50`.

### Summary

This report repairs the auditable ideation record without converting planned
tests into claimed runtime evidence. AC-1 and AC-3 through AC-6 now each name
their durable design source and the implementation/validation proof still
required; native identity remains behind the disposable feasibility gate.
