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

## Problem

Pending reviews need truthful attention without forcing the operator to hunt for their originating work or silently losing gates whose provenance is unavailable.

## Required outcome

A gate with verified v1 origin appears only in its originating tab. A gate with no usable origin remains globally visible. The rail reconciles the provider-owned set of open gates so resolution updates or removes the row.

## Origin-carrier spike — negative finding

There is no usable v1 origin carrier in the current baseline. `grout/gate.go`
reads a decision-log path and brief frontmatter, and its `gateInfo` contains
only that path plus display fields. `grout/rows.go` serializes the same shape.
The rail's `GateEvent` accepts only `log_path`, title, stage, round, and
recommendation, then keys an update by `log_path`; unknown JSON fields are
ignored. None is a Zellij origin, so simply adding an upstream `origin` key
would not bind a row.

The available Review & Gate v1 probe is also not that contract. Its `Briefing`
has `type`, `version`, `id`, `question`, `artifacts`, and opaque `context`
nodes. A context node is raw JSON; the probe has no `ZaphodOrigin` type,
schema check, canonical placement, or native-identity validation. It can
preserve an unknown JSON member, but declares, validates, and emits no origin.
Its open envelope is only `{status,url,log}`, and its terminal output is only
`{resolution,log}`; it does not offer a configured open-gate snapshot. An
origin extension is therefore technically storable but not a gate-skill-
provided, authoritative carrier.

Nor may the missing field be filled with the current Zellij values. The rail
receives `TabUpdate.tab_id`, but the native-identity feasibility record calls a
raw tab ID a locator, not ownership: it has not proved session incarnation,
replacement, or ID-reuse safety. A path, process CWD, title, tab display
position, session name, active client, pane ID, or guessed tab is therefore
not an origin fallback.

This is a source-level spike, not a passing live claim: no actual current v1
payload carries a declared, validated origin to exercise. The result is
decisive for this slice. Zaphod must project every current gate globally and
must not invent a carrier. Implementation remains blocked until the gate skill
accepts and emits the contract below, and the identity authority named by that
contract is available.

## Proposed approach

### One configured provider snapshot

Use one configured v1 gate provider, not a glob, a decision-log parser, or a
rail-maintained gate server. The provider owns a complete snapshot of its
currently open gates. Each source record needs a provider-owned opaque gate
key, display fields, and optional origin. A successful snapshot replaces that
provider's previous open set; a gate absent from the next successful complete
snapshot is removed. A malformed, partial, failed, or duplicate-key snapshot
does not remove anything: retain the last accepted set and surface the source
failure through the provider's normal diagnostics. This replaces neither the
current probe's `{status,url,log}` lifecycle nor its review authority until the
gate skill accepts the extension.

The trusted source configuration identifies exactly one registered provider,
its accepted contract major, and its bounded `list_open_gates` operation. It
does not let event data select an executable, a URL, shell arguments, or a
filesystem glob. The adapter runs at most one refresh per provider, gives that
operation a fixed deadline, and emits nothing on timeout or provider error; the
last accepted open set remains visible until a valid replacement arrives.

The rail receives one atomic `GateSnapshotEvent` for every accepted result,
including `gates: []`; it never infers a snapshot boundary from per-gate rows.
An event is valid only when its provider equals the configured provider, its
contract major and `complete: true` are accepted, its revision is newer than
the last accepted revision, and every gate has a nonempty unique provider key
plus a nonempty display title. The contract defines a strictly ordered provider
revision. A malformed, partial, duplicate-key, unconfigured-provider, or older
event is rejected without changing the accepted set. Thus an older result
cannot recreate a gate that a newer provider snapshot removed.

The proposed external gate-skill contract is deliberately small and is not an
assumed Zaphod input:

```text
GateSnapshotEvent {
  kind: "gate_snapshot",
  provider: ProviderId,
  contract_major: 1,
  revision: ProviderOrderedRevision,
  complete: true,
  gates: [OpenGateV1]
}
OpenGateV1 {
  gate_key: ProviderOwnedOpaqueId,
  display: { title: NonEmptyString, detail: String },
  origin?: ZaphodOriginV1
}
ZaphodOriginV1 {
  schema: "zaphod.gate-origin.v1",
  zellij_identity: CanonicalZellijTabIdentityV1
}
```

At gate creation, the authorized Zellij identity authority may supply one
optional immutable `ZaphodOriginV1`; the gate skill validates its declared
shape, preserves it verbatim, and delivers it in every later open snapshot. It
may not reconstruct origin from a later path, title, CWD, client, or tab
lookup. The identity authority, not the gate skill or rail, defines the
canonical identity token, session/incarnation scope, freshness check, and exact
equality rule. An origin is stale when a fresh authority query cannot establish
the same unique current identity. `ZaphodOriginV1` is optional. Until this
external contract exists, every record is equivalent to `origin: absent`.

Before implementation, the gate skill must publish a versioned
`GateOriginFixtureBundleV1` with a source revision and digest. It contains raw
valid, absent, malformed, duplicate-key, changed, empty, and ordered/stale
snapshot cases plus the accepted identity fixtures. Zaphod consumes those raw
records unchanged; it does not author expected origin values in its own tests.

### Reconcile provider truth before projecting it

For each configured provider, validate the complete snapshot and reconcile it
by `(provider, gate_key)`, never by `log_path`. Reconciliation updates an
existing open gate in place, adds a new key, and removes only keys absent from
a later valid complete snapshot. It neither folds a decision log nor decides
whether a provider action resolved a gate.

Each rail instance then projects the reconciled open set locally:

- A gate is **bound** only when its optional origin is schema-valid and its
  complete canonical `zellij_identity` equals this instance's current,
  supported `CanonicalZellijTabIdentityV1` exactly.
- A gate is **global** when origin is absent, malformed, unsupported, stale,
  mismatched, or the instance has no supported identity. Here *stale* means a
  fresh identity-authority query did not establish one unique equal identity.
  Global gates render in every rail; they are not dropped or guessed into one
  tab.
- A bound gate renders in its one equal-identity rail and in no other rail.
  Comparison is whole-token equality. There is no normalization, prefix,
  substring, path, CWD, title, display-position, active-tab, or guessed-pane
  matching.

This keeps provenance honest even when the provider cannot bind a gate. It
also lets a provider resolution become visible without a rail callback: the
next authoritative open snapshot updates or removes the row.

### Keep review authority outside the rail

This task supplies visibility and reconciliation only. It does not add an
inline verdict, write a decision log, launch a gate server, derive a brief from
a log path, or choose a reviewer surface. The v1 review-surface handoff owns
accepted review routing and lifecycle. Legacy `pz` work proves only that a
provider-owned server can resolve a gate; its rail-issued approve path is not
an architecture to reuse.

### Existing pure seams

Implementation should extend the existing pure boundaries rather than build a
parallel rail model:

- Add `GateSnapshotEvent` to `parse_agent_event`, then replace the gate branch
  of `apply_agent_event`/`upsert` with a pure `reconcile_open_gates` operation
  keyed by `(provider, gate_key)` that accepts only a newer provider revision.
- Add a source-side snapshot-event builder beside `BuildGateRow`; it emits an
  explicit empty accepted set as `gates: []`. `GateFromLog` and
  `gateFromBrief` must not derive origin from their path input or pretend that
  one legacy row is a complete provider snapshot.
- Keep `GateEvent` as the post-reconciliation display model, then reuse
  `gate_row_line` and `gate_row_detail` to render the resulting projection.
- Do not use `brief_path_for_log` or `decide_rail_click` as an origin or v1
  review-routing mechanism. The later v1 handoff task owns any accepted action.

## Riskiest unproven mechanism

The riskiest mechanism is the real v1 gate skill carrying a canonical optional
`ZaphodOriginV1` that can be compared to a live Zellij tab identity without
guessing. It is unproven and currently absent. The first test below is the
smallest end-to-end invalidator: it feeds the actual provider-emitted carrier
into two independently identified rails. If the carrier cannot be emitted,
validated, or exactly matched, the bound branch is unsupported and the
implementation must retain global-only projection rather than substitute a
heuristic.

## Acceptance criteria

### Offline

**AC-O1 — The source contract is optional, external, and fail closed.** A
configured v1 provider record with no `ZaphodOriginV1`, a malformed origin, an
unknown schema, or an unavailable local Zellij identity produces one global
row. It never produces a tab-bound row from `log_path`, CWD, title, tab
position, session name, active client, or pane ID.

Verified by: the pinned, digest-checked `GateOriginFixtureBundleV1` and its
separate Zellij-identity fixture define accepted identity values. The projection
test consumes the raw bundle records, gives absent and malformed cases their
deliberately misleading paths, CWDs, titles, positions, and IDs, then asserts
the bundle's global outcome. The current no-origin `GateRow` fixture is an
independent baseline: all of its gates remain global.

**AC-O2 — Exact origin eliminates foreign-tab hunting without hiding global work.** Given one complete provider snapshot with three provider-owned keys — `gate-a` carrying tab identity A, `gate-b` carrying distinct identity B, and `gate-global` with no origin — rail A renders exactly `{gate-a, gate-global}` and rail B renders exactly `{gate-b, gate-global}`. The foreign-bound row count is zero in each rail, while the global row count is one in each rail.

Verified by: a two-rail projection test consumes canonical identity values and
expected row sets from the pinned `GateOriginFixtureBundleV1`, not strings
written by the reconciler. Its independent baseline is the current global
projection, where each rail would expose one foreign bound gate; the measured
foreign-row count must fall from one to zero without reducing global rows.

**AC-O3 — A provider's open-set truth updates and removes rows.** From the
same valid provider source, a later complete snapshot changes `gate-b`, omits
resolved `gate-a`, and retains `gate-global`. Reconciliation updates B in
place, removes A, and retains the global row. A malformed, partial, or failed
second snapshot leaves the previously accepted rows intact. A valid but older
revision also cannot restore A.

Verified by: an adapter/reconciler integration test uses versioned complete,
empty, changed, invalid, and older snapshots from the pinned fixture bundle and
checks provider gate-key sets before and after each result. It checks that an
invalid event removes nothing and an older revision cannot resurrect a key.

**AC-O4 — Visibility does not acquire decision or routing authority.**
Processing an open-gate snapshot makes no verdict request, decision-log write,
server launch, direct `subspace-tui` float, or tab-switch request. A bound row
and a global row use the same projection path; origin only filters visibility.

Verified by: a fake provider plus fake Zellij sink records no action arguments
while reconciliation and rendering run. The pinned
`GateOriginFixtureBundleV1` supplies the provider data; the check observes
calls at the process boundary, not source-text matches. Review opening is
covered only by the separate v1 handoff task.

### Captain-live

**AC-C1 — An operator sees the right pending work, then truthful resolution.**
After the disposable-profile gate and the v1 contracts pass, CL opens two real
Zellij tabs whose independently queried identities match the provider's A and
B records. Each tab shows only its bound gate plus the global gate. A reviewer
resolves one gate independently in the provider; after the next valid provider
snapshot, that gate disappears from its origin tab without changing the other
bound or global row. Zaphod does not open, route, or decide that review.

Verified by: CL drives the two-tab disposable-profile drill and records the
provider's before/after open snapshots, the pinned carrier fixture version,
and the profile's live Zellij inventory. The drill rejects a path-, CWD-,
title-, or guessed-tab explanation for a row's placement.

## Test plan

1. **Run the origin-carrier invalidation first, after `7h` and the external
   v1 contracts are available.** In one disposable profile, create two rails
   with independently queried canonical identities. Capture one real gate
   created in A with its accepted origin and one gate with absent or corrupt
   origin. Carry the raw records through source, grout, and pipe without
   reconstruction. Assert A alone renders the bound gate and both rails render
   the global gate. No carrier, non-exact match, or non-unique identity is a
   failed bound-path prerequisite and leaves only global projection. Stop this
   spike here; it neither resolves nor opens a review.
2. Add pure parsing, origin-classification, and per-provider reconciliation
   tests using the external contract's valid, absent, malformed, duplicate,
   changed, and out-of-order snapshots. Include hostile path, CWD, title,
   position, session, active-client, and pane-ID values to prove no inference
   path exists.
3. Run provider-adapter-to-rail integration against fake provider and Zellij
   sinks. Check full-snapshot replacement, empty-snapshot removal,
   invalid-snapshot retention, stale-revision rejection, two-instance row sets,
   and zero decision/routing side effects at the sink.
4. Run the focused Rust and Go suites, then the disposable-profile harness.
   Do not create a new profile or touch standing Zellij configuration. No KDL
   dump fixture belongs in this task: tab identity arrives from the identity
   authority, and any identity implementation that parses dumps must use the
   real single-line dump shape required by that authority's own gate.
5. Prepare the two-tab CL drill for AC-C1 only after the offline checks pass.
   The drill observes a provider-side resolution performed independently of
   Zaphod; it does not click an inline rail verdict, drive a direct float, or
   invoke the separate review-surface handoff.

## Documentation change

When the behavior ships, replace README.md's **Agent & gate rows** wording
with: “A v1 pending gate with a verified provider origin appears only in its
originating rail. A gate without a usable origin appears globally in every
rail. Zaphod never infers origin from a path, CWD, or title, and the provider
retains review decisions and routing.” Keep the existing review-action wording
separate until the v1 review-surface handoff has an accepted contract.

## Out of scope

Implementing or backfilling the v1 gate skill's origin carrier; creating a
Zellij identity authority, marker, controller, managed tab, hub, or tmux
driver; filesystem globbing or log folding; legacy `pz` server launch and
rail-issued approve; direct review floats, inline verdicts, decision-log
writes, reviewer routing, prewarming, or review-surface cleanup. This task
also does not turn a raw tab ID, pane ID, session name, path, CWD, title, or
display position into identity.

## Stage Report: ideation

- DONE: Spike the actual v1 origin carrier and record the negative result.
  Current `gateInfo`, `GateRow`, and `GateEvent` carry no origin; the available
  Review & Gate v1 probe preserves opaque context but emits only
  `{status,url,log}`/`{resolution,log}` and validates no origin. Current Zellij
  `tab_id` remains a locator rather than proven ownership. No live Zellij drill
  was run because there is no carrier to exercise. This is the fail-closed
  evidence required by AC-O1.
- DONE: Define configured-provider reconciliation and exact/global projection.
  A valid, ordered, atomic provider snapshot reconciles by provider-owned gate
  key; gate creation captures an authority-owned origin verbatim, and only
  whole-token identity equality binds a row. Every absent, malformed,
  unsupported, or mismatched origin remains globally visible. The two-rail
  value and update/removal evidence are specified in AC-O2 and AC-O3.
- DONE: Bound provider authority, multi-tab evidence, and documentation.
  The ACs measure foreign-row elimination and post-resolution truth across two
  tabs, prohibit rail verdicts/routing, name the external contract prerequisite,
  and propose the README behavior change. AC-O4 keeps provider authority out of
  the rail; AC-C1 reserves the actual two-tab operator proof for CL.

### Summary

The current system has no v1 gate-origin carrier, so it cannot truthfully bind
a gate to a tab today. This design keeps every such gate visible globally,
proposes an optional gate-skill-owned `ZaphodOriginV1` contract, and permits a
bound row only after exact supported identity equality. Provider snapshots, not
rail actions, update or remove open gates; review decisions and routing stay
with the provider.
