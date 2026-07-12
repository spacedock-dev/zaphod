---
title: Pending gates appear where the work came from
status: ideation
source: captain direction 2026-07-11; Sprint 2 outcome shaping
sprint: s2-dependable-per-tab-attention-loop
group: walking-skeleton
sprint-readiness: defer
blocked-on: gate-skill-publisher-contract-and-bb-sidecar
blocked-reason: Implementation waits for an accepted external gate-skill publisher contract and bb’s accepted fresh-tab sidecar handoff; neither is invented locally.
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

## Captain-directed re-ideation (cycle 3)

The global-first outcome remains right, but the prior body invented a
`GateSnapshotV1` contract inside Zaphod. It described a publisher, open-set
semantics, revisions, and fixtures that no gate skill has supplied. That is
not a design shortcut: it would make Zaphod the authority for gate truth.

This task is therefore not implementation-ready until two named dependencies
exist. It records the exact consumer work Zaphod will do afterward, without
claiming the missing provider contract already exists.

## Problem

An operator needs one current list of pending gates in the rail and needs a
gate to disappear only after the gate system has really resolved it. Today the
rail receives one legacy log-derived row at a time. It has no provider-owned
open-set boundary, so it cannot distinguish a resolved gate from a publisher
that merely stopped reporting.

## Required outcome

**Trigger:** a registered gate skill publishes a later valid complete view of
its open gates while the bb sidecar is serving a managed rail. **Visible
result:** every rail that receives that publication displays the same global
pending gates; a gate disappears only when the gate skill's later valid view
omits it after its own resolution. **Reproducible proof:** a real gate-skill
fixture opens and resolves one gate through its own workflow, and the built
sidecar and rail visibly add then remove that gate in an isolated tmux-hosted
Zellij run.

“Global” is deliberate. This slice makes no claim that a path, CWD, title,
pane, tab, session, or client identifies the work's origin.

## Evidence and hard dependencies

### What exists today

Review & Gate v1 is a portable review contract, not an open-gate service. Its
scope assigns workflow position, approver authority, and routing to workflow
tooling; it defines immutable Briefings and ordered review entries, but no
publisher invocation, provider-wide open set, snapshot revision, liveness, or
Zellij placement. The current Subspace gate-loop recipe likewise launches and
waits for one gate folder; it does not enumerate all live gates.

Zaphod's current Go path reads one decision-log path and brief frontmatter into
one `GateRow`, and the Rust rail upserts it by `log_path`. Neither is a complete
provider view or a resolution transition. Zaphod must not turn those paths,
globs, or a scan of state files into a substitute provider.

### Prerequisite GP-1 — Spacedock gate-skill publication contract (external)

**Owner:** the Spacedock gate skill/workflow tooling that owns gate lifecycle;
not Zaphod, `grout`, or the Subspace reviewer app. **Status:** absent at this
revision. It must be accepted outside this repository before this task starts
implementation.

GP-1's delivery artifact must contain all of the following:

- one registered provider identity and an invocation/subscription lifecycle
  owned by the gate skill. Zaphod may attach only to the documented receiver;
  it never chooses a program, URL, shell arguments, glob, log path, or polling
  cadence from event data, and it never starts or restarts the publisher;
- a versioned, complete open-gate publication. Each accepted publication says
  exactly which gates are open for that provider scope, including an explicit
  empty set. It distinguishes a complete view from an unavailable, partial, or
  failed observation;
- a provider-owned opaque, stable `gate_key` for each logical gate, display
  fields, and ordering/revision rules. A later publication can update a gate
  without changing its key, and an older or duplicate publication cannot
  resurrect a resolved gate;
- liveness and failure semantics: consumers can recognize an unavailable or
  stale publisher without treating silence as a resolution. The contract says
  how a new publisher epoch or restart becomes authoritative;
- a versioned fixture bundle, owned and released with the provider, that gives
  raw valid, update, empty, older, duplicate, malformed, partial, unavailable,
  and restarted-publication cases plus their expected provider-owned open sets;
  and
- one executable provider integration proof: create/open a real fixture gate,
  resolve it through the gate skill's normal result path, and observe a later
  complete publication that omits that same stable `gate_key`. A hand-authored
  Zaphod pipe, a decision-log scan, or a fixture that merely deletes a row does
  not satisfy this proof.

The external owner chooses the serialization and exact field names. This record
does not name a local `GateSnapshotV1` or make Zaphod's parser the source of
the contract. At GP-1 acceptance, its pinned contract revision and fixture
bundle become this task's external test baseline.

### Prerequisite BB-1 — accepted sidecar handoff

`bb` must have an accepted and implemented fresh-tab handoff before this task
starts. The `Alt Shift z` entry creates the tab, observes the exact resident
rail, and starts the private native `zaphod subscribe` sidecar with its target
tuple. This task adds a gate-provider consumer behind that private sidecar; it
does not create another launcher, public `grout` command, daemon, controller,
lease, or test profile.

BB-1 is a lifecycle boundary, not a tab-placement claim. The existing named
`agent-event` bridge is session-wide transport. Its verified rail target only
governs sidecar lifetime; it does not turn the gate's global publication into a
tab-owned one.

## Proposed Zaphod approach after GP-1 and BB-1

The private `zaphod subscribe` process receives only the registered gate
skill's already-owned publication through GP-1's documented boundary. It does
not invoke, discover, or repair the gate provider. It rejects a message that
does not meet the accepted external contract and retains the last accepted
provider set across malformed, partial, stale, duplicate, unavailable, or
silent input. A later externally valid complete publication, including the
provider's explicit empty set, is the only removal authority.

The Rust rail adds a pure provider-set reconciler keyed by the external
`(provider, gate_key)` pair. One accepted complete publication atomically
replaces that provider's projection. It extends `parse_agent_event`,
`apply_agent_event`, the gate display helpers, and `decide_rail_click`; the Go
side extends the internal emitter behind `zaphod subscribe`. The bridge remains
transport only.

Every receiving rail renders that reconciled provider set globally. Snapshot-
derived rows are display-only in this slice: selecting one performs no float,
verdict, decision-log write, provider command, or tab switch. The separate v1
review-surface handoff owns any later trusted action. Legacy log-path rows stay
separate and must not be promoted into provider snapshots.

Tab-bound placement is deferred. This task neither reads nor stores origin
metadata and never guesses it from a decision-log path, brief path, CWD, title,
tab position, raw ID, pane, session, or active client.

## Riskiest unproven mechanism

The critical joint is not a Rust row: it is GP-1's real resolution transition
surviving bb's target-bound sidecar into the visible rail. First, the external
provider must run its own fixture proof: open one gate, resolve it through its
normal result path, and publish the later complete state that omits its stable
key. Then an isolated tmux-hosted Zellij run starts the normal fresh tab and
private sidecar, consumes those published records, and visibly adds then
removes the fixture title.

The check fails if Zaphod invokes the provider, scans a decision log, sends a
hand-issued pipe, infers tab identity, uses a custom PTY/lease harness, or
turns a gap in one-shot rows into removal.

## Acceptance criteria

### Offline

**AC-O1 — Only a gate-skill-owned complete publication changes pending-gate
truth.** Against the GP-1 fixture bundle, the valid open publication creates
exactly the externally expected provider/key set; the later valid update and
empty publications produce the externally expected replacement sets. Older,
duplicate, malformed, partial, unavailable, and silent-source cases leave the
last accepted set unchanged until the external liveness/restart rule accepts a
new complete publication.

Verified by: Go adapter and Rust reconciler tests consume the pinned GP-1 raw
fixture bundle and assert its expected provider/key sets after each record. No
expected open state, revision, or liveness value is authored by the code under
test.

**AC-O2 — Global-first projection exposes pending work without fabricated
ownership.** Delivering one accepted GP-1 publication to two independent rail
states yields the same provider/key set in both, including after update and
empty transitions. Extra path-, CWD-, title-, pane-, tab-, session-, and
client-shaped data has no effect on either set.

Verified by: Rust tests feed one external fixture sequence to two rail states
and measure their rendered provider/key sets against the fixture baseline;
hostile locator-shaped values are supplied only as ignored extras.

**AC-O3 — Zaphod does not invoke or decide for the provider.** Processing each
GP-1 fixture record causes zero gate-skill process starts, provider commands,
direct reviewer floats, decision-log writes, verdicts, or Zellij tab switches.
Selecting a snapshot-derived row is a no-op.

Verified by: an adapter/rail boundary test uses provider, review, workflow, and
Zellij fakes to record all calls while it consumes the external fixture bundle;
every forbidden call count is zero. Legacy log-path coverage remains separate.

**AC-O4 — A real provider resolution reaches the built rail through BB-1.**
In a temporary-root tmux-hosted Zellij server, the normal fresh-tab entry
starts the accepted private sidecar. GP-1's executable fixture opens then
resolves one gate and publishes its later complete state; the resident rail
visibly gains then loses the provider's fixture title. Standing Zellij and tmux
configuration remains unchanged, and the temporary roots are cleaned up.

Verified by: the GP-1 provider integration proof plus the existing
tmux-hosted isolated-profile smoke style capture the provider publication,
native target state, and rail screen before open, after open, and after
resolution. Neither test hand-writes a pipe nor uses a custom PTY/lease.

### Captain-live

**AC-C1 — An operator sees current pending work and its real resolution without
hunting.** With one real gate open in the configured gate skill, the managed
rail shows its global row. After the provider resolves that gate independently,
the next accepted complete publication removes the row. No tab-bound claim,
manual pipe, or rail-issued review decision is involved.

Verified by: a captain drill records the provider's before/after publications,
the accepted sidecar target, and the rail's before/after display. The v1
review-surface handoff, not this drill, owns reviewer placement and decision
interpretation.

## Test plan

1. **Gate the work on the external invalidator.** GP-1 first proves its
   publisher-owned invocation and open-to-resolved fixture transition. If it
   is absent or fails, keep this task in ideation; do not build a Zaphod
   surrogate.
2. Once BB-1 and GP-1 are accepted, run AC-O4's end-to-end provider → private
   sidecar → rail smoke first in an isolated tmux-hosted Zellij profile.
3. Add Go and Rust tests for every GP-1 fixture record: acceptance, atomic
   replacement, empty removal, last-good retention, and externally specified
   liveness/restart behavior.
4. Add two-rail global-projection and no-action tests, including hostile
   locator-shaped extras. Do not add a tab identity field or a review route.
5. Run focused Go and Rust suites, the isolated smoke, then AC-C1 only after
   the offline packet passes. Do not mutate standing Zellij or tmux
   configuration.

## Documentation change

After GP-1 and this consumer ship, replace README's Agent & gate rows wording
with: “The configured gate skill publishes its own complete open-gate view.
Pending gates appear globally in each receiving rail and disappear only after
a later valid provider publication omits them. Zaphod does not infer a gate's
tab from a path, CWD, title, or multiplexer ID, and it does not decide reviews.”

## Out of scope

Defining, publishing, or invoking the gate skill's contract inside Zaphod;
Review & Gate v1 changes; decision-log folding, filesystem discovery, legacy
`pz` launch, rail-issued approve, direct review floats, inline verdicts, or
review routing; tab-bound origin metadata or any identity heuristic; a public
`grout` command, hub/controller, pooling, leases, custom PTY infrastructure,
7h, 4d, or standing configuration mutation.

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

## Stage Report: ideation (cycle 3)

- SKIPPED: Obtain a gate-skill-owned snapshot contract rather than inventing one locally.
  `review-and-gate.md`, the `spacedock-subspace` command surface, and a targeted publisher scan expose no provider-wide publisher; GP-1 names the external contract and fixture bundle required before implementation.
- DONE: Declare the accepted bb subscriber dependency and global-first behavior.
  BB-1 is now an explicit precondition: the accepted fresh-tab handoff starts the private target-bound sidecar, while all receiving rails project gate publications globally with no tab inference.
- SKIPPED: Prove a real resolution transition and fixture ownership boundary.
  The required proof is correctly assigned to GP-1's provider-owned executable fixture; it cannot be run truthfully until that external publisher and its fixture bundle exist.

### Summary

The staff finding was correct: Zaphod had specified a gate publisher it does
not own. This cycle removes that fabricated contract, names GP-1 and BB-1 as
hard prerequisites, and leaves a narrow global-first consumer design ready for
review only after those prerequisites are accepted.
