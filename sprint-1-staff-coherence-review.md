---
title: Staff software engineering coherence review for Sprint 1
status: ideation
source: captain-requested independent full-sprint review after Sprint 1 ideation, 2026-07-11
started: 2026-07-11T05:59:12Z
completed:
verdict:
score: 1.0
worktree:
issue:
pr:
mod-block:
id: 40c5qx8k5spz1y3fsrghyc5n
---

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

### Cross-packet interface and ownership assessment

| Seam | Assessment and required gate decision |
| --- | --- |
| Foreground profile -> CLI and spike | The profile proves one foreground client; the CLI adds `ZAPHOD_BIN`, while the spike needs a second independently foregrounded attachment. Freeze a small profile-test interface: session/namespace metadata, profile root, client PGID, attachment command inputs, and teardown ownership. The foreground lane owns the base lifecycle, the CLI lane owns the candidate binary, and the spike owns the second PTY client. |
| CLI -> binding core | `Handshake`, correlated envelopes, and mutation states agree, but the CLI currently owns only `health` and unsupported `binding.inspect`; no packet names the Go package/process that implements the registry verbs and is invoked by `cmd/zaphod`. Assign that owner at the contract gate (within the existing `grout` module) and map each `binding.*` command to it before implementation. |
| Handshake -> driver capabilities/errors | The CLI's baseline transport capabilities and the contract's native identity capabilities are compatible but not yet one advertised vocabulary. Publish one v1 matrix for transport, binding, and driver capabilities plus canonical malformed-envelope, protocol-mismatch, and correlation error tags. Zellij identity capabilities must remain absent until the spike proves them. |
| Binding contract -> Zellij spike | The binding UUID, schema, incarnation, and marker concepts are compatible, but `zaphod.binding.v1/<binding_id>` must be explicitly mapped to the marker-pane configuration and fresh-query evidence at the native-identity gate. The spike, not the portable core, owns proof of namespace, incarnation, marker persistence, and queryability. |

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
