---
title: Pending gates appear where the work came from
status: ideation
source: captain direction 2026-07-11; Sprint 2 outcome shaping
sprint: s2-dependable-per-tab-attention-loop
group: walking-skeleton
sprint-readiness: ready
blocked-on:
blocked-reason:
score: 0.97
started: 2026-07-12T00:03:20Z
completed:
verdict:
worktree:
issue:
pr:
mod-block:
id: s92rm4v2pz0m23memjg9xha2
---

## Captain-directed re-ideation

This cycle supersedes the prior tab-origin design. It made Sprint 2 wait for a
canonical Zellij tab-identity authority and a Review & Gate extension that do
not exist. The first useful outcome is smaller: a pending gate published by
one configured gate skill appears truthfully in every receiving rail, and a
later complete provider snapshot removes it after the provider resolves it.

Tab-bound placement remains desirable later, but no path, CWD, title, pane ID,
tab ID, session name, or active client may stand in for origin while the
provider has not supplied one.

## Problem

An operator can finish reviewing a gate while the work is still invisible in
the rail, or keep seeing a gate that the provider has already resolved. The
operator then has to hunt for the gate and cannot tell whether a row is current.

## Required outcome

**Trigger:** the configured gate skill reports its complete set of open gates
while a Zaphod rail is running. **Visible result:** each receiving rail shows
those pending gates globally; after the provider reports a later complete set
without a resolved gate, that row disappears. **Reproducible proof:** an
isolated tmux-hosted Zellij run sends two real provider snapshots through the
private subscription path and observes a distinctive row appear and disappear
in the built rail.

“Global” here means every rail that receives the common event. It deliberately
does not claim that a guessed Zellij tab owns the work.

## Current evidence and boundary

The present Go adapter reads one decision-log path and brief frontmatter into
one GateRow. The rail parses one kind: gate event and upserts it by log path;
neither side has an open-set boundary or a removal event. A one-shot row cannot
truthfully say that a missing later row is resolved.

Review & Gate v1 is not the missing source contract. Its portable Briefing and
review log intentionally describe one review opportunity and its result; they
do not enumerate all currently open gates or define Zellij placement. The gate
skill integration must own that operational snapshot. Zaphod consumes it; it
does not scan logs, glob files, or infer open state.

The planned private zaphod subscribe sidecar is the common-bus seam. The
managed-tab entry starts it only after its exact resident rail has been
observed. This task adds a gate-provider adapter behind that private process;
there is no public grout command, controller, hub, lease, custom PTY, or new
user entry.

## Proposed approach

### One configured provider and one complete snapshot

For this first slice, one registered gate-skill provider has one fixed,
checkout-controlled invocation or subscription. Event data cannot choose an
executable, URL, shell argument, glob, or filesystem path. The provider
publishes only a complete open-set snapshot:

~~~text
GateSnapshotV1 {
  kind: "gate_snapshot"
  provider: RegisteredProviderId
  revision: monotonically increasing unsigned integer
  complete: true
  gates: [{
    gate_key: provider-owned opaque nonempty ID
    title: nonempty display text
    detail: display text
  }]
}
~~~

The gate skill owns gate_key, display values, revision ordering, and the
meaning of open. Zaphod accepts only the registered provider, a newer complete
revision, and unique nonempty gate keys and titles. A duplicate, older,
malformed, partial, or failed source result emits no replacement; the last
accepted set remains visible. An explicit valid empty gates list is the
provider’s truthful “none open” signal.

The sidecar serializes valid snapshots onto the existing common agent-event
bridge. No individual row says it is a snapshot, and Zaphod never treats a
missing one-shot gate event as a resolution.

### Global rail projection, separated from review authority

The rail gains a pure provider-snapshot reconciler keyed by
(provider, gate_key). A newer valid snapshot atomically replaces that
provider’s open set. It emits the resulting display rows in the existing gate
section. All rails that receive the same snapshot project the same rows, so
there is no tab routing decision to get wrong.

Snapshot-derived rows are display-only in this slice. Selecting one produces
no direct float, verdict, decision-log write, provider command, or tab switch.
The existing legacy log-path row behavior remains unchanged for legacy events;
the separate v1 review-surface handoff owns any future trusted open action for
provider snapshot rows.

This extends the existing pure seams rather than adding a parallel runtime:

- Go: the internal gate row builder and emitter gain a complete-snapshot
  adapter behind private zaphod subscribe.
- Rust: parse_agent_event, apply_agent_event, the gate display helpers, and
  decide_rail_click gain a distinct snapshot-derived gate model and pure
  replacement operation.
- The existing named agent-event bridge remains transport only. It carries no
  authority to decide a review.

### Later tab-bound placement

A later gate-skill contract may attach an optional immutable origin token to
an open-gate record. The gate skill, not Zaphod, must define and validate that
token and provide its lifecycle. A separate task can then prove exact
placement against a supported native identity authority.

This slice neither reads nor stores origin metadata. Until that later proof,
all current gates remain global. There is no heuristic fallback from a
decision-log path, brief path, CWD, title, tab position, raw tab ID, pane ID,
session name, or active client.

## Riskiest unproven mechanism

The risk is not rendering one row; it is preserving provider truth across a
real open-to-resolved transition. The smallest invalidator runs the built
private zaphod subscribe process against the gate skill’s versioned fixture
provider in an isolated tmux-hosted Zellij profile. Snapshot 41 supplies one
distinctive open gate; snapshot 42 is a valid empty set. The actual rail must
show the row after 41 and no longer show it after 42.

The run fails if it uses a hand-issued pipe instead of the sidecar, a
decision-log scan, a custom PTY harness, a guessed tab identity, or a
one-shot row whose absence is interpreted as removal.

## Acceptance criteria

### Offline

**AC-O1 — A complete provider snapshot is the sole source of pending-gate
truth.** Given the gate skill’s versioned fixture snapshots, valid revision 41
with keys alpha and beta creates exactly those two rows; valid revision 42
with only updated beta removes alpha and updates beta; valid revision 43 with
an empty list removes beta. An older, partial, malformed, or duplicate-key
snapshot after revision 41 changes nothing.

Verified by: Go adapter and Rust reconciliation tests consume the provider’s
fixture records and assert accepted provider/key sets after each record. The
expected keys and labels come from the provider fixture, not from the
reconciler under test.

**AC-O2 — Global-first projection exposes pending work without fabricated tab
ownership.** The same accepted fixture snapshot delivered to two independent
rail states yields the same global alpha/beta set in both. After revision 42,
both show only beta; after revision 43, neither shows a gate. Hostile extra
fields containing a path, CWD, tab title, raw tab ID, pane ID, session name,
or active-client value do not change either set.

Verified by: Rust tests feed the raw provider fixture plus hostile ignored
extras into two rails and measure their rendered gate-key sets. The independent
baseline is the fixture’s open-set sequence; no Zaphod-authored placement
value is used.

**AC-O3 — Visibility acquires no review or routing authority.**
Snapshot-derived rows render but select to no action. Processing valid,
invalid, changed, and empty snapshots issues no provider command, direct
review float, decision-log write, verdict, or Zellij tab-switch request.

Verified by: a rail/action-boundary test records calls to fake provider and
Zellij sinks while it processes the provider fixture and selects each row; all
recorded action counts remain zero. Legacy log-path row coverage remains
separate.

**AC-O4 — The smallest real source-to-rail path reflects resolution.**
In a temporary-root tmux-hosted Zellij server, the normal fresh-tab entry
starts the built private sidecar. A fixture gate-skill process supplies
snapshots 41 then 42; the real sidecar emits the common event and the built
resident rail visibly adds then removes the fixture title. The run leaves
standing Zellij configuration untouched and cleans up the temporary tmux and
Zellij roots.

Verified by: the existing isolated-profile smoke style captures native pane
state and visible screen before, after 41, and after 42. It invokes neither a
custom PTY/lease harness nor a hand-written pipe command.

### Captain-live

**AC-C1 — An operator sees current pending work without hunting, then sees
provider resolution truthfully.** With one real gate from the configured gate
skill open, the operator sees its global row in the managed rail. The provider
resolves it independently; its next valid complete snapshot removes that row.
No tab-bound claim, gate action, or manual pipe invocation is needed.

Verified by: a captain drill records the provider’s before/after snapshots and
the rail’s before/after display in a managed tab. The review-surface handoff
task, not this drill, owns opening the reviewer UI or interpreting its result.

## Test plan

1. Run AC-O4’s open-to-empty real sidecar/rail invalidator first. It uses the
   standard isolated tmux-hosted profile, not 7h or any custom PTY machinery.
2. Add provider-fixture parsing and complete-snapshot validation tests:
   revision ordering, empty removal, changed rows, duplicate keys, partial
   data, malformed data, and last-good retention.
3. Add pure Rust reconciliation and two-rail global-projection tests,
   including hostile locator-shaped extras and non-actionable selection.
4. Run focused Go and Rust suites, then the isolated profile smoke. Do not
   mutate standing Zellij or tmux configuration.
5. After the offline packet passes and the gate skill has a real open gate,
   run AC-C1. Do not substitute a legacy direct float or an inline verdict.

## Documentation change

When this behavior ships, replace README’s Agent & gate rows wording with:
“A configured gate provider publishes complete open-gate snapshots. Pending
gates appear globally in each receiving rail and disappear only after a later
valid provider snapshot omits them. Zaphod does not infer a gate’s tab from a
path, CWD, title, or multiplexer ID, and it does not decide reviews.”

## Out of scope

Tab-bound origin metadata or an identity authority; inferring placement from
paths, CWDs, titles, tab positions, raw IDs, panes, sessions, or clients;
filesystem globs, decision-log folding, legacy pz server launch, rail-issued
approve, direct review floats, inline verdicts, review routing, provider
discovery, a public grout command, a hub/controller, pooling, leases, custom
PTY infrastructure, 7h, 4d, and standing configuration mutation.

## Stage Report: ideation

- DONE: Spike the actual v1 origin carrier and record the negative result.
  Superseded as a delivery prerequisite: the absence of a carrier now selects
  global-first projection rather than blocking the operator loop.
- DONE: Define configured-provider reconciliation and exact/global projection.
  Superseded in part: complete provider snapshots and reconciliation remain;
  exact tab-origin projection moves to a later separately proven contract.
- DONE: Bound provider authority, multi-tab evidence, and documentation.
  Provider authority remains outside the rail; the previous two-tab ownership
  proof is replaced by a two-rail global projection proof.

### Summary

The initial cycle correctly found no usable origin carrier, but it made that
absence a Sprint 2 blocker. Cycle 2 retains the fail-closed finding and ships
the available value first: truthful global pending gates.

## Stage Report: ideation (cycle 2)

- DONE: Reduce the gate projection to a global-first end-user path.
  The required outcome and AC-O1 through AC-C1 now use one configured
  provider’s complete open-set snapshots; rows render globally and disappear
  only on a later valid omission.
- DONE: Remove 7h, raw tab identity, and unavailable external-contract prerequisites.
  The record names Review & Gate v1’s intentionally absent open-set and
  placement contracts, excludes locator heuristics, and uses the normal
  tmux-hosted isolated smoke instead of 7h, leases, or a custom PTY.
- DONE: Name the smallest gate-skill source and rail event proof.
  GateSnapshotV1 is a provider-owned complete snapshot, carried by the
  private zaphod subscribe/common agent-event seam; AC-O4 exercises snapshot
  41 then 42 through the actual sidecar and built rail.
- DONE: Map each acceptance criterion to an independently observable proof.
  AC-O1 → provider fixture revisions and test-plan step 2; AC-O2 → two-rail
  global set comparison and step 3; AC-O3 → the fake provider/Zellij action
  sink and non-actionable selection in step 3; AC-O4 → the real sidecar/rail
  invalidator in step 1; AC-C1 → the provider/rail before-and-after captain
  drill in step 5. Expected rows and revisions originate in the gate-skill
  fixture, not the Zaphod reconciler.

### Summary

This task is ready to deliver the gate half of Sprint 2 without pretending it
knows tab provenance. The gate skill remains the authority for open-state
snapshots and any later origin metadata; Zaphod is a global-first projection
and never a reviewer or router.
