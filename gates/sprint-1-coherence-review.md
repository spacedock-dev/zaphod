---
review: v1
kind: sprint-coherence
workflow: agent-rail-dev
status: reviewer-approved
captain-decision: pending
source: captain-requested independent full-sprint review after Sprint 1 ideation, 2026-07-11
source-entity-id: 40c5qx8k5spz1y3fsrghyc5n
reviewed-tasks:
  - foreground-attached-client-profile
  - zaphod-native-cli-skeleton
  - managed-view-driver-contract
  - zellij-managed-identity-feasibility
---

# Sprint 1 coherence review gate record

## Problem

Sprint 1 now has four independently authored ideation packets: foreground
attached-client profile repair, native zaphod CLI ownership, the portable
binding/driver contract, and Zellij identity feasibility. Review them as one
delivery slice before implementation so strong local designs do not conceal a
broken critical path, mismatched interface, unsafe parallelism, or a user
journey that promises features Sprint 1 does not actually deliver.

## Review scope

Act as an independent staff software engineer. Review `docs/roadmap.md`,
`docs/zaphod-workspace-architecture.md`, and the four active Sprint 1 task
records: `foreground-attached-client-profile`, `zaphod-native-cli-skeleton`,
`managed-view-driver-contract`, and `zellij-managed-identity-feasibility`.
You may use the prior `managed-view-sprint-review` as historical evidence, but
reassess it against the current packets rather than treating it as authority.
Do not edit product code or product documentation.

## Expected outcome

Return one staff recommendation: approve, approve with concrete reframing, or
reject. Name the minimum critical path, safe concurrency, cross-packet
interfaces and ownership, integration risks, required gates/evidence, and
explicitly parked work. Include a concise `End-user value and journey at Sprint
1 exit` section that distinguishes the trustworthy foundation users gain from
the managed-tab, keybinding, pane-adoption, hub, dock, and provider behavior
that remains unshipped.

## Acceptance criteria

**AC-1 — The proposed delivery sequence is coherent.**
Verified by: a cited dependency graph that identifies each required gate and
does not schedule an interactive/native action before its prerequisites.

**AC-2 — Cross-packet seams are mutually compatible.**
Verified by: a cited interface table covering artifact path, handshake,
capabilities, identity, mutation results, recovery, and feasibility inputs.

**AC-3 — Sprint 1's user value is honest and actionable.**
Verified by: a bounded end-user journey that maps each promised outcome to a
Sprint 1 exit criterion and names unsupported behavior as out of scope.

**AC-4 — The recommendation gives executable next decisions.**
Verified by: concrete preserve/change/park actions and evidence required at
each subsequent gate; no generic approval language.

## Test plan

Perform a read-only cross-document review. Trace every claimed dependency to a
task record or roadmap rule, test interface claims against the records, and
separate current evidence from planned future checks. No product build, live
Zellij drill, or standing configuration mutation is part of this review.

## Out of scope

Product implementation, modifying task designs, approving human gates,
executing native feasibility experiments, or changing the roadmap.

## Staff coherence review

### Recommendation

**Approve with concrete reframing.** The packets implement the roadmap's
foundation-and-feasibility slice without prematurely shipping managed-view
behavior. Before implementation dispatch, however, the shared contract gate
must assign three currently implicit integration seams: the Go binding-core
owner, the wire capability/error vocabulary, and the multi-client disposable
profile handoff. These are bounded decisions, not new product scope.

### Critical path, gates, and safe concurrency

`foreground-attached-client-profile` validation and merge is the entry gate;
it proves the foreground PTY, raw input, cleanup, and global isolation before
any native drill. The roadmap then requires a shared contract freeze for the
artifact path, protocol major, full opaque session identity, and typed result
envelope. Only then may these three lanes run together:

```text
foreground profile gate
        -> shared contract gate
           -> native CLI artifact/profile wiring
           -> portable registry + fake-adapter suite
           -> disposable Zellij identity/marker spike
        -> native-identity reconciliation
        -> integrated contract, process, native-drill, and isolation gate
```

This follows `docs/roadmap.md`'s dispatch order and the task prerequisites:
the CLI's candidate-profile check and the two-client spike both wait for the
foreground gate, while the registry's fake suite may proceed after the
contract freeze. No captain drill, production keybinding, or controller work
may move ahead of those gates. The three lanes are logically concurrent, but
the CLI and spike will both touch the disposable-profile/test surface; assign
one profile-integration owner or isolate their helpers before merge so this is
also source-level safe concurrency.

**Sources:** `docs/roadmap.md` — “Delivery rules,” “Dispatch and merge order,”
and “Sprint gates”; `foreground-attached-client-profile.md` — “Sprint role”
and AC-O1–AC-O5; `zaphod-native-cli-skeleton.md` — “Test plan”;
`managed-view-driver-contract.md` — “Dependency boundary”; and
`zellij-managed-identity-feasibility.md` — “Contract-freeze prerequisites.”

### Cross-packet interface and ownership assessment

| Seam | Assessment and required gate decision | Sources |
| --- | --- | --- |
| Foreground profile -> CLI and spike | The profile proves one foreground client; the CLI adds `ZAPHOD_BIN`, while the spike needs a second independently foregrounded attachment. Freeze a small profile-test interface: session/namespace metadata, profile root, client PGID, attachment command inputs, and teardown ownership. The foreground lane owns the base lifecycle, the CLI lane owns the candidate binary, and the spike owns the second PTY client. | `foreground-attached-client-profile.md` — “Proposed approach”; `zaphod-native-cli-skeleton.md` — “Ownership and artifact boundary”; `zellij-managed-identity-feasibility.md` — “Disposable offline-first harness.” |
| CLI -> binding core | `Handshake`, correlated envelopes, and mutation states agree, but the CLI currently owns only `health` and unsupported `binding.inspect`; no packet names the Go package/process that implements the registry verbs and is invoked by `cmd/zaphod`. Assign that owner at the contract gate (within the existing `grout` module) and map each `binding.*` command to it before implementation. | `zaphod-native-cli-skeleton.md` — “Versioned native seam”; `managed-view-driver-contract.md` — “Minimal driver-neutral seam and native CLI packet.” |
| Handshake -> driver capabilities/errors | The CLI's baseline transport capabilities and the contract's native identity capabilities are compatible but not yet one advertised vocabulary. Publish one v1 matrix for transport, binding, and driver capabilities plus canonical malformed-envelope, protocol-mismatch, and correlation error tags. Zellij identity capabilities must remain absent until the spike proves them. | `zaphod-native-cli-skeleton.md` — “Versioned native seam”; `managed-view-driver-contract.md` — “Minimal driver-neutral seam and native CLI packet” and “Typed result and mutation model”; `zellij-managed-identity-feasibility.md` — “Contract-freeze prerequisites.” |
| Binding contract -> Zellij spike | The binding UUID, schema, incarnation, and marker concepts are compatible, but `zaphod.binding.v1/<binding_id>` must be explicitly mapped to the marker-pane configuration and fresh-query evidence at the native-identity gate. The spike, not the portable core, owns proof of namespace, incarnation, marker persistence, and queryability. | `managed-view-driver-contract.md` — “Durable identity and registry invariants” and “View identity, invalidation, and explicit recovery”; `zellij-managed-identity-feasibility.md` — “Proposed approach” and “Evidence matrix.” |

### Material integration risks and gate handling

1. The missing binding-core command owner is the only present ownership gap
   that can block the Sprint 1 exit promise of create/inspect/reconcile/repair
   and remove. Resolve it before code rather than letting the CLI skeleton or
   feasibility helper acquire registry policy incidentally.
2. Native-ID reuse is correctly treated as unsafe, but the feasibility packet
   both requires a reused numeric ID and says bounded non-observation is not
   proof. The native-identity gate must define a deterministic allocator
   witness or return an explicit negative/unsupported result; absence of
   observed reuse cannot turn the gate green.
3. Keep the create-before-persist barrier authoritative. A timeout or killed
   helper is `Indeterminate` until fresh inspection finds exactly one marker;
   it must never be retried as a second create or repaired from a name.

### Preserve, change, and park

Preserve the foreground PTY canary and global-isolation discipline; the Go
native-artifact boundary; opaque session identity; reverse uniqueness; and
fail-closed inspect/rebind/repair semantics. Change only the shared-gate
addendum above and designate the profile integration handoff. Park the
production managed view/controller, `Alt Shift z`, `Alt /`, pane adoption,
workspace hub, dock, provider adapters/notify ingress, tmux product driver,
and installer rollout exactly as the roadmap requires.

**Sources:** `docs/roadmap.md` — “Delivery rules,” “Sprint 2 — managed Zellij
entry and guarded toggle,” “Sprint 3 — explicit pane adoption,” “Later
delivery,” and “Parked outside the release path.” The architecture source is
`docs/zaphod-workspace-architecture.md` — “Non-goals”; each Sprint 1 packet's
“Out of scope” supplies the task-level boundary.

### End-user value and journey at Sprint 1 exit

At a passing Sprint 1 exit, an operator can use the documented foreground
disposable profile to type real terminal input, verify safe cleanup, and
inspect a repository-owned, versioned native `zaphod` candidate without
changing standing Zellij files. The binding foundation can safely expose
explicit, recoverable records only after the contract and native-identity
gates accept its evidence; it remains fail-closed rather than guessing from
names or reused IDs.

This is deliberately not a managed-workspace user journey yet. The managed
view, production keybindings, pane adoption, workspace hub, portable dock,
and AgentsView, review, and notify providers remain unshipped; a temporary
spike marker or test key is evidence, not a user-facing entry path.

**Sources:** `docs/roadmap.md` — “Sprint 1 — managed-view foundation and
feasibility” (Goal, Sprint gates, and Exit criteria), “Sprint 2 — managed
Zellij entry and guarded toggle,” “Sprint 3 — explicit pane adoption,” and
“Later delivery”; `docs/zaphod-workspace-architecture.md` — “User experience,”
“Managed view and pane adoption,” “Dock,” and “Provider adapters”; and the
four Sprint 1 packets' “Out of scope.”

### Actionable staff recommendation

Approve implementation only after the captain records the three-part shared
contract addendum (binding-core owner, capability/error matrix, and
multi-client profile interface). If any native-identity row fails, advertise
no Zellij identity capability and return the representation—not a fallback
controller—to the shared contract gate.

## Stage Report: ideation

- DONE: Trace the full Sprint 1 critical path, gate order, and safe concurrency against the authoritative roadmap rather than reviewing packets in isolation.
  The review's dependency graph follows the roadmap dispatch order and distinguishes logical parallel lanes from their shared profile/test integration surface.
- DONE: Reconcile the CLI, binding-contract, foreground-profile, and native-feasibility seams; name only material contradictions, missing owners, or unproved claims.
  The review identifies one missing command-owner decision, one capability/error freeze, one profile handoff, and the ID-reuse proof condition; it finds the marker and result-model directions otherwise compatible.
- DONE: State an honest end-user value and journey at Sprint 1 exit, distinguishing delivered foundation from intentionally unshipped product behavior, then give one actionable staff recommendation.
  The Sprint 1 journey is limited to safe disposable infrastructure and a versioned candidate artifact; the final recommendation conditions implementation on a concrete shared-gate addendum.

### Summary

The four packets form a coherent foundation if the roadmap gates remain hard:
foreground PTY evidence first, a narrow shared contract freeze, three bounded
lanes, then native-identity reconciliation. The recommendation is approval
with concrete reframing so the native CLI, portable registry, and disposable
Zellij spike converge on one owned interface without falsely shipping managed
workspace behavior.

## Stage Report: ideation (cycle 2)

- DONE: Trace the full Sprint 1 critical path, gate order, and safe concurrency against the authoritative roadmap rather than reviewing packets in isolation.
  Added a source block citing the roadmap's delivery rules, dispatch order, sprint gates, and each lane's prerequisite packet section.
- DONE: Reconcile the CLI, binding-contract, foreground-profile, and native-feasibility seams; name only material contradictions, missing owners, or unproved claims.
  Added a per-row source column that cites the relevant foreground, CLI, binding-contract, and feasibility packet sections.
- DONE: State an honest end-user value and journey at Sprint 1 exit, distinguishing delivered foundation from intentionally unshipped product behavior, then give one actionable staff recommendation.
  Added explicit roadmap, architecture, and task-boundary citations for the exit journey and the preserve/change/park recommendation.

### Summary

This evidence-only repair makes the existing recommendation auditable against
the authoritative roadmap, architecture, and four task records. It leaves the
recommendation, review scope, product files, and task designs unchanged.

## Fresh independent re-review — Sprint 1 coherence (cycle 3)

### Recommendation

**APPROVE** — I independently re-read the current four packets, the roadmap,
the architecture, and the earlier staff review. The revised packets now close
the prior material design seams without promoting an unproved Zellij behavior
to product scope. The approval is for the Sprint 1 ideation contract and its
gated implementation sequence, not for a controller, a production keybinding,
or a live-identity claim.

### Prior material findings: closure check

| Earlier material finding | Independent closure assessment | Evidence |
| --- | --- | --- |
| Binding-core ownership and command routing were implicit. | **Closed.** `grout/internal/bindingcore` owns binding state, registry policy, recovery, and the injected driver; `grout/internal/zaphodcli` performs one typed dispatch; `grout/cmd/zaphod` remains a process and wire boundary. The five currently eligible binding discriminants have named service destinations, while `binding.ensure-managed-view` is reserved and unadvertised. | `managed-view-driver-contract.md` — “Shared-contract addendum — Sprint 1 gate” / “Go binding-core owner and command dispatch”; `zaphod-native-cli-skeleton.md` — “Ownership and artifact boundary.” |
| Capability, protocol-error, and mutation semantics could diverge. | **Closed.** One protocol-v1 matrix defines the initial three transport capabilities, delayed binding/driver advertisement, `MalformedEnvelope`, `ProtocolMismatch`, `CorrelationMismatch`, `Unsupported`, and the only legal `Changed`, `Unchanged`, and `Indeterminate` outcomes. The CLI and feasibility packet consume the same matrix. | `managed-view-driver-contract.md` — “Protocol-v1 capability, error, and mutation matrix”; `zaphod-native-cli-skeleton.md` — “Versioned native seam”; `zellij-managed-identity-feasibility.md` — “Contract-freeze prerequisites.” |
| The foreground profile did not define a safe multi-client handoff or teardown boundary. | **Closed.** The immutable `ProfileLeaseV1` publishes only after the foreground PGID and session readiness checks. The foreground packet owns A, the root, and final teardown; feasibility owns B's PTY, PGID, controller artifacts, evidence, and release; the CLI owns only the lease-scoped candidate binary. B must release before final teardown, and each independent run receives a fresh lease. | `foreground-attached-client-profile.md` — “Proposed approach” and “AC-O2 — the attached client owns the foreground PTY process group before a lease is exposed”; `managed-view-driver-contract.md` — “Disposable-profile lease and marker-pane handoffs”; `zellij-managed-identity-feasibility.md` — “Shared lease and marker-tuple contract.” |
| Binding identity was not tied to a queryable native marker. | **Closed as a feasibility-gated representation.** The shared tuple maps binding UUID, incarnation nonce, schema, and canonical controller URL into KDL configuration; a fresh controller must structurally query every tab and cross-check the live pane inventory. A name, active client, or bare tab ID cannot substitute. | `managed-view-driver-contract.md` — “Disposable-profile lease and marker-pane handoffs”; `zellij-managed-identity-feasibility.md` — “Shared lease and marker-tuple contract” and “Evidence matrix.” |
| Reused native IDs could be treated as safe when reuse was merely not observed. | **Closed.** The feasibility harness must deterministically force and observe reuse. If it cannot, it returns correlated `Unsupported` with `Unchanged` and Zellij advertises none of the four identity/inventory/marker/create capabilities; it does not use a name, position, or fallback controller. | `managed-view-driver-contract.md` — “Protocol-v1 capability, error, and mutation matrix”; `zellij-managed-identity-feasibility.md` — “Evidence matrix,” “AC-O4 — Session replacement and deterministic native ID reuse cannot impersonate the original view,” and “Contract-freeze prerequisites.” |

### Coherent delivery and remaining material issue

The dependency order is now hard and mutually compatible:

```text
foreground PTY + ProfileLeaseV1 gate
  -> shared binding/CLI contract freeze
     -> CLI candidate/profile wiring
     -> bindingcore registry + fake-adapter suite
     -> disposable two-client Zellij feasibility spike
  -> native-identity reconciliation and integrated evidence gate
```

This follows `docs/roadmap.md` — “Dispatch and merge order” and “Sprint
gates.” The three post-freeze lanes may proceed independently because their
shared lease rules give the profile lane final teardown ownership and require
fresh leases per independent run. An integrated run serializes use of one
lease and releases B before cleanup.

**Remaining material issue: none in the ideation seams.** The still-unproved
native Zellij identity mechanism is deliberately an implementation gate, not
an omitted design decision. Before any Zellij identity capability is
advertised, the native-identity gate must either pass every two-client,
marker-query, replacement, deterministic-reuse, and crash-window row or emit
the specified negative `Unsupported` result with no identity capability. That
is the required next decision; it must not be replaced by an active-client,
display-name, cwd, or tab-position fallback.

### Acceptance-criteria audit trail

| Acceptance criterion | Fresh review conclusion and supporting record evidence |
| --- | --- |
| **AC-1 — The proposed delivery sequence is coherent.** | The foreground gate precedes the contract freeze; only then do the CLI, portable core, and feasibility lanes start; native identity reconciles them before integration. Supporting task records: `foreground-attached-client-profile.md` — “Sprint role”; `managed-view-driver-contract.md` — “Dependency boundary” and “Shared-contract addendum — Sprint 1 gate”; `zellij-managed-identity-feasibility.md` — “Contract-freeze prerequisites.” Authoritative sequence: `docs/roadmap.md` — “Dispatch and merge order.” |
| **AC-2 — Cross-packet seams are mutually compatible.** | Ownership, envelope semantics, lease handoff, marker query, and negative-capability behavior use one vocabulary without a duplicate policy owner. Supporting task records: `managed-view-driver-contract.md` — “Go binding-core owner and command dispatch,” “Protocol-v1 capability, error, and mutation matrix,” and “Disposable-profile lease and marker-pane handoffs”; `zaphod-native-cli-skeleton.md` — “Ownership and artifact boundary” and “Versioned native seam”; `foreground-attached-client-profile.md` — “Proposed approach”; `zellij-managed-identity-feasibility.md` — “Shared lease and marker-tuple contract.” |
| **AC-3 — Sprint 1's user value is honest and actionable.** | The planned exit provides a safe foreground disposable profile, a lease-scoped inspectable candidate, and a recoverable binding foundation only where the driver evidence permits it. It does not promise a shipped managed view, production `Alt Shift z`/`Alt /`, adoption, hub, dock, provider, tmux product driver, or global installation. Supporting task records: `foreground-attached-client-profile.md` — “Captain-live” and “Out of scope”; `zaphod-native-cli-skeleton.md` — “Acceptance criteria” and “Out of scope”; `zellij-managed-identity-feasibility.md` — “Captain-live” and “Out of scope.” Authoritative boundary: `docs/roadmap.md` — “Sprint 1 — managed-view foundation and feasibility” and `docs/zaphod-workspace-architecture.md` — “Non-goals.” |
| **AC-4 — The recommendation gives executable next decisions.** | Preserve the frozen owner/matrix/lease/tuple rules; dispatch implementation only in the stated gate order; record either all required Zellij evidence or the typed negative capability decision; park controller and product behavior until Sprint 2. Supporting task records: `managed-view-driver-contract.md` — “Test plan” and “Out of scope”; `zaphod-native-cli-skeleton.md` — “Test plan”; `zellij-managed-identity-feasibility.md` — “Evidence matrix,” “Test plan,” and “Contract-freeze prerequisites.” |

### Updated Sprint 1 end-user journey

At Sprint 1 exit, an operator can launch the documented foreground disposable
profile, type into a real terminal, and verify cleanup without changing
standing Zellij files. After the ready lease exists, the operator can inspect
the repository-owned candidate at `$PROFILE_ROOT/bin/zaphod protocol`; it is a
versioned, scoped diagnostic surface, not an installed product launcher. The
binding core can create, inspect, reconcile, repair, and remove records only
through exact identity and explicit recovery rules. If the Zellij feasibility
gate is negative, the same interface reports `Unsupported` with no native
mutation instead of guessing ownership.

The ordinary managed-workspace journey remains later work: no production
managed tab, keybinding, layout toggle, pane adoption, workspace hub, dock,
provider behavior, tmux driver, or installer rollout ships in Sprint 1. This
matches `docs/roadmap.md` — “Exit criteria” and “Sprint 2 — managed Zellij
entry and guarded toggle,” plus `zellij-managed-identity-feasibility.md` —
“Documentation change” and “Out of scope.”

## Stage Report: ideation (cycle 3)

- DONE: Independently re-review the revised four-packet Sprint 1 design against the roadmap, architecture, and historical staff findings.
  All five former material seams now have a named owner, one contract, or a fail-closed feasibility disposition; no current packet claims a production Zellij behavior from unproved evidence.
- DONE: Verify cross-packet lifecycle and native-identity safety boundaries.
  `ProfileLeaseV1` separates A/B/client-binary ownership, the KDL tuple binds UUID and incarnation to a fresh query, and deterministic ID reuse now has only a proof-or-`Unsupported` outcome.
- DONE: Return one recommendation, explicit AC-1 through AC-4 evidence citations, and an honest Sprint 1 exit journey.
  The result is **APPROVE** for the gated ideation contract; native identity remains a hard implementation gate and product controller behavior remains parked.

AC-1 evidence: `foreground-attached-client-profile.md` — “Sprint role”; `managed-view-driver-contract.md` — “Dependency boundary”; `zellij-managed-identity-feasibility.md` — “Contract-freeze prerequisites.”
AC-2 evidence: `managed-view-driver-contract.md` — “Shared-contract addendum — Sprint 1 gate”; `zaphod-native-cli-skeleton.md` — “Ownership and artifact boundary”; `foreground-attached-client-profile.md` — “Proposed approach”; `zellij-managed-identity-feasibility.md` — “Shared lease and marker-tuple contract.”
AC-3 evidence: `foreground-attached-client-profile.md` — “Captain-live”; `zaphod-native-cli-skeleton.md` — “Acceptance criteria”; `zellij-managed-identity-feasibility.md` — “Out of scope.”
AC-4 evidence: `managed-view-driver-contract.md` — “Test plan”; `zaphod-native-cli-skeleton.md` — “Test plan”; `zellij-managed-identity-feasibility.md` — “Evidence matrix.”

### Summary

The revised packets are coherent enough to leave ideation. They preserve the
roadmap's hard gates, converge on one owner and result vocabulary, and make
the unresolved Zellij mechanism either demonstrably safe or explicitly
unsupported before later product work can depend on it.
