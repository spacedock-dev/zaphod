---
title: One v1 review opens and returns cleanly
status: ideation
source: captain direction 2026-07-11; Sprint 2 outcome shaping
sprint: s2-dependable-per-tab-attention-loop
group: walking-skeleton
sprint-readiness: ready
blocked-on:
blocked-reason:
score: 0.95
started: 2026-07-12T00:14:07Z
completed:
verdict:
worktree:
issue:
pr:
mod-block:
id: qtwk8waj9t38n2d0daktnz8d
---

## Problem

The present direct float path can orphan a review pane and does not respect the v1 split between workflow-owned routing and reviewer-owned decisions.

## Required outcome

For one v1 gate, the gate skill delegates an accepted review surface to Zaphod. Zaphod opens the reviewer UI visibly beside the originating work and returns the operator cleanly; the gate skill validates and routes the result, and the rail later shows provider truth.

## Ideation boundary

Start with the simplest visible surface, not hidden prewarming. Define an accept/result handshake; let Zaphod keep only runtime surface correlation and lifecycle; preserve durable origin in v1 context; and close only the exact accepted surface after exit and result handoff. A direct skill float is allowed only when Zaphod was unavailable before acceptance. Do not let Zaphod interpret approve, revise, or hold; do not duplicate a review surface or change foreign tabs.

## Reconnaissance and live host pre-spike

The legacy rail click is not a v1 handoff to extend. `ClickAction::FloatGate`
derives a brief from `log_path`, invokes `open_command_pane_floating`, discards
its returned `PaneId`, and has no `CommandPaneExited` subscription. It cannot
identify the pane it opened, close only that pane, or distinguish a reviewer
exit from a gate decision. The legacy path remains evidence of a visible
surface only; it has no authority for v1 routing or results.

The locally available Review & Gate v1 probe is also not the missing contract.
Its reviewer emits `review-v1-open-result` or `review-v1-result` with a
briefing and log, but no gate-skill handoff ID, receiver capability, or
result-delivery capability. Zaphod must not scrape command-pane stdout or
infer correlation from a briefing ID or log path. A gate skill must provide a
one-shot, authenticated result channel before this slice can implement.

### Live host pre-spike — passed, with a decisive duplicate finding

On 2026-07-12, an isolated Zellij 0.44.3 server used its own temporary
config, data, cache, home, and socket roots. Its two tabs were `origin`
(`tab_id` 0) and `foreign` (`tab_id` 1). A fixture-only command,
`new-pane --floating --close-on-exit --tab-id 0 -- /bin/sh -c 'sleep 2'`,
returned `terminal_2`; `action list-panes -a -g -s -t` showed that float only
under `origin`, with no pane or layout change in `foreign`. After exit, the
same live inventory contained only the two original terminals.

Issuing that identical native launch twice returned `terminal_3` and
`terminal_4`, both floating simultaneously in `origin`. Zellij therefore
provides a usable exact runtime pane handle and clean native exit, but it does
not deduplicate launches. The fixture's raw tab ID was an isolated test target,
not a product origin or ownership token. The production path must use
`open_command_pane_floating_near_plugin` from the selected receiver and an
idempotent Zaphod registry; it must never turn a tab ID, position, path, title,
or active client into durable provenance.

## Proposed approach

### One receiver, one explicit acceptance

A review begins at the rail that the operator selected. A bound gate can be
selected only in the rail whose separately validated v1 origin projection
renders it; an unbound global gate uses the rail the operator clicked. That
rail creates an opaque, volatile `SurfaceReceiverV1` capability. It denotes
one resident Zaphod receiver, not a durable tab identity, and expires with the
attempt. The gate skill returns a request only to that receiver; it must not
broadcast a named Zellij pipe and let every rail decide whether to launch.

The required gate-skill-to-Zaphod contract is:

```text
ReviewSurfaceOfferV1 {
  schema: "zaphod.review-surface.v1",
  handoff_id: GateSkillOpaqueId,
  receiver: SurfaceReceiverV1,
  provider: RegisteredProviderId,
  launch: GateSkillTrustedLaunchV1,
  result_capability: GateSkillOneShotCapability
}

ReviewSurfaceReceiptV1 {
  handoff_id: GateSkillOpaqueId,
  receiver: SurfaceReceiverV1,
  status: accepted | declined | indeterminate,
  surface_id?: ZaphodOpaqueRuntimeId
}
```

`launch` is a registered-provider command vector or equivalent trusted launch
capability, never a shell string from a gate row. The gate skill validates the
gate, reviewer, launch, and route before offering it. Zaphod validates the
schema, provider registration, and receiver liveness. It accepts only after
`open_command_pane_floating_near_plugin` returns `Some(PaneId::Terminal(id))`.
The receipt exposes `surface_id`, never a native pane ID or a tab locator.

An explicit `declined` means Zaphod opened no pane, so the gate skill may take
its direct fallback once. An `accepted` receipt forbids direct fallback for
that `handoff_id`. A timeout, transport failure, or lost reply is
`indeterminate`, not a decline: the gate skill retries or asks the receiver's
status endpoint using the same ID, but it does not start a second surface.
This rule closes the lost-acknowledgment duplicate race.

### Zaphod owns only runtime surface lifecycle

Zaphod keeps an in-memory registry keyed by `handoff_id`:

```text
Opening | Open { surface_id, terminal_pane_id } | Exited | Closed
```

It reserves an ID before calling the host. A replay during `Opening` or `Open`
returns the existing receipt and launches nothing. A fresh review requires a
new gate-skill handoff ID after a closed attempt. `open_command_pane_floating`
is not used: `open_command_pane_floating_near_plugin` attaches the visible
surface to the receiver's own tab even if another tab later has focus. The
launch context carries the opaque `{handoff_id, surface_id}` pair.

Zaphod subscribes to `CommandPaneExited` and `PaneClosed`. It closes a command
pane only when the event's terminal ID and context match the live registry
entry, then removes that entry on the matching close. An unknown, stale, or
foreign event is ignored. A manual close becomes lifecycle evidence only; it
never closes another pane or triggers a relaunch. If Zaphod loses its runtime
registry, it returns `indeterminate` and does not recover by scanning paths,
titles, output, or raw tab IDs. The first slice requires an explicit new
handoff after visible cleanup.

### The gate skill owns result validation and routing

The trusted launcher posts a provider result directly to the gate skill:

```text
GateReviewResultV1 {
  handoff_id: GateSkillOpaqueId,
  result_capability: GateSkillOneShotCapability,
  provider_result: ProviderOwnedOpaqueResult
}
```

The gate skill verifies the capability, correlates the result, persists any
provider truth, and decides whether approve, revise, hold, or another outcome
changes workflow state. Zaphod receives or emits only an opaque lifecycle
notice such as `SurfaceExited { handoff_id, surface_id, exit_code? }`.
`CommandPaneExited`, an exit code, a log path, and a reviewer process's stdout
are never a decision. A close without a valid result leaves the gate skill's
gate unchanged.

The current v1 reviewer does not yet carry `handoff_id` or
`result_capability`; a gate-skill-owned wrapper or an additive reviewer
contract must supply them. That change belongs to the gate skill/reviewer,
not Zaphod.

### Existing seams and excluded legacy path

Implementation extends a small pure `ReviewSurfaceRegistry` for reservation,
receipt replay, exact exit matching, and removal. It connects that registry to
the existing `Sidebar` state, `ZellijPlugin::update`, and Zellij's
`open_command_pane_floating_near_plugin`, `CommandPaneExited`, and `PaneClosed`
interfaces. `ClickAction::FloatGate`, `brief_path_for_log`, the legacy
`GateEvent`, and `open_command_pane_floating` are not v1 routing seams. They
must neither derive a v1 artifact nor decide a v1 result.

## Riskiest unproven mechanism

The riskiest mechanism is an authoritative gate skill issuing one idempotent,
receiver-scoped offer whose reviewer result returns through a one-shot
gate-skill capability. The current v1 probe proves neither the receiver path
nor result correlation. The native pre-spike proves that Zellij can place and
close a known fixture pane, and it proves that native launches duplicate when
the caller does not suppress them; it does not prove the v1 contract.

The smallest end-to-end invalidator must run first after the gate skill freezes
the contract. In one disposable two-tab profile, have rail A mint one receiver
while B is a foreign tab. Deliver the same valid offer twice to A, using a
fixture reviewer that posts one opaque result through the gate skill's result
capability and exits. Live pane inventory must show exactly one reviewer pane
in A, zero in B, and no second direct-fallback pane. After exit, exactly that
pane must disappear; the gate skill must record exactly one accepted result and
make the routing decision itself. A missing receiver, missing result
capability, ambiguous receipt, mismatched exit context, or a second pane
invalidates the design and leaves the review unlaunched or explicitly
indeterminate. No heuristic fallback is allowed.

## Acceptance criteria

### Offline

**AC-O1 — One accepted handoff eliminates duplicate and foreign review
surfaces.** Given the external gate-skill fixture's one receiver-scoped offer,
two identical deliveries yield one `surface_id` and one launch. A distinct
receiver never receives that pane.

Verified by: a registry-plus-driver test uses the gate skill's versioned offer
fixture and a fake Zellij sink. It measures launch count `1`, replay count `1`,
and foreign launch count `0`. The independent native baseline is this
ideation's live double-launch observation, where the equivalent unguarded
request produced two panes.

**AC-O2 — Zaphod cleans up only the surface it accepted.** A matching
`CommandPaneExited` context closes its tracked terminal pane; an unknown pane,
wrong context, or unrelated manual close issues no close command for any
accepted surface.

Verified by: a driver test supplies recorded Zellij event fixtures with native
pane IDs and context generated by the launch fixture, then observes exact
`close_terminal_pane` calls at the fake host boundary. The expected IDs and
contexts originate outside the registry under test.

**AC-O3 — Review results retain gate-skill authority.** A valid opaque result
reaches the gate skill once; Zaphod makes no verdict, routing, decision-log,
or provider-state call. A surface exit without that result makes no workflow
change.

Verified by: a fake gate-skill endpoint verifies its one-shot capability and
records result delivery, while a fake Zaphod boundary records no decision or
routing call. The provider's result fixture, not Zaphod code, supplies the
decision-shaped payload.

**AC-O4 — Fallback happens only before known acceptance.** An explicit decline
permits one direct fallback. Accepted, replayed, exited, and indeterminate
handoffs invoke no direct fallback for that ID.

Verified by: a gate-skill protocol test drives decline, accepted, duplicate,
lost-reply, and exit traces against an external direct-launch fake and measures
its invocation count. The fake's count, rather than the handoff prose,
establishes the boundary.

### Captain-live

**AC-C1 — An operator opens one visible reviewer and returns without tab
hunting.** After `7h`, the gate projection contract, and this v1 contract are
available, CL opens a real pending gate from rail A while B remains foreign.
One provider reviewer appears beside A's work, its result returns to the gate
skill, and exiting it leaves A and B's ordinary panes intact with no duplicate
or fallback surface.

Verified by: CL drives the disposable-profile drill. Before, during, and after
the run, the harness captures live `list-panes` and layout state, the gate
skill's accepted receipt and result receipt, and the provider's later open-set
projection. The pending-gate task owns row placement and later truth; this
task owns the one visible surface and its handoff.

## Test plan

1. **Run the contract invalidator first, after the external v1 contract is
   available.** Use the two-tab disposable-profile drill described in
   *Riskiest unproven mechanism*: one receiver, one offer delivered twice,
   one fixture result, one exit. It must inspect live panes, not generated KDL
   or a transcript. A failed prerequisite leaves the surface unlaunched or
   indeterminate; it never substitutes a raw tab ID, path, title, CWD, or
   broadcast pipe.
2. **Keep the completed host pre-spike as a narrow regression fixture.** Its
   two-tab native run proves controlled fixture placement and clean exit, and
   its duplicate result proves why the registry is mandatory. It is not an
   acceptance test for origin, routing, or the gate-skill result contract.
3. Add pure registry tests for reservation, same-ID replay, explicit decline,
   indeterminate reply loss, matching and mismatching exit contexts, manual
   close, and post-close new-ID behavior.
4. Add gate-skill/Zaphod integration tests using the versioned offer and result
   fixtures. Assert one registered command-vector launch, one result delivery,
   zero Zaphod decision calls, and zero direct fallback after acceptance.
5. Run focused Rust tests and the disposable-profile harness before CL's drill.
   The actual reviewer contract must fail loudly when unavailable; no silent
   skip may mark the value check green.
6. Run AC-C1 only after the offline packet passes. Do not replace it with a
   direct legacy float, an inline verdict, or a hand-authored pane inventory.

## Documentation change

When the v1 contract ships, replace README.md's review-action wording with:
“For an accepted v1 review handoff, Zaphod opens one visible provider-owned
reviewer in the rail that requested it and closes only that surface after it
exits. The gate skill validates results and routes decisions; Zaphod never
interprets approve, revise, or hold. A direct fallback is allowed only before
Zaphod accepts the handoff.” Keep the existing legacy float wording until the
contracted path replaces it.

## Out of scope

Implementing the gate skill's offer, result capability, reviewer wrapper, or
origin/identity authority; broadcasting a surface request; a hub, managed-tab
controller, tmux driver, or a new tab identity scheme; deriving a reviewer
from a log path or brief; inline verdicts, decision-log writes, or provider
routing; stdout scraping; hidden prewarm, pools, or automatic relaunch after
Zaphod state loss. The first slice also does not modify the legacy direct
float path or treat it as a fallback after acceptance.

## Stage Report: ideation

- DONE: Define the accepted review-surface handoff and result contract so gate skill routing stays separate from Zaphod surface ownership.
  The record requires a one-receiver offer, post-open acceptance receipt, opaque gate-skill result capability, and lifecycle-only Zaphod notices.
- DONE: De-risk same-tab surface correlation, exit/close handling, and no-duplicate/no-foreign-tab behavior with the smallest live spike.
  An isolated two-tab Zellij 0.44.3 run placed and removed one fixture float only in `origin`; two identical launches produced two floats, proving the registry must enforce idempotence.
- DONE: Keep the first slice visible and simple: fallback only before acceptance, no hidden prewarm, and no Zaphod decision semantics.
  Explicit decline is the sole fallback permission; accepted and indeterminate handoffs never fall back, and result interpretation remains entirely with the gate skill.

### Summary

The v1 path is a narrow surface handoff, not a new review controller. Zaphod owns one visible, receiver-local pane and its exact runtime cleanup; the gate skill owns launch authority, result validation, and every workflow decision. The host spike confirms placement and close mechanics but exposes native duplicate launch behavior, so an idempotent accepted-handoff registry is mandatory before implementation.

## Stage Report: ideation (cycle 2)

- DONE: Retain the approved v1 handoff design and add explicit Stage Report evidence mappings for every acceptance criterion, especially AC-O2 through AC-O4.
  AC-O1 → *One receiver, one explicit acceptance*, the native pre-spike, and test-plan steps 1–4: the external offer fixture, fake Zellij sink, and observed two-launch baseline establish one accepted receipt, one launch, and zero foreign launches.
  AC-O2 → *Zaphod owns only runtime surface lifecycle* and test-plan steps 1 and 3: the `CommandPaneExited`/`PaneClosed` context-match rule extends the existing capture-ID-then-close seam recorded in `gate-click-subspace-tui-pane-never-closes.md:21-88`; recorded event fixtures drive exact close calls.
  AC-O3 → *The gate skill owns result validation and routing* and test-plan step 4: the versioned opaque-result fixture and one-shot capability reach the fake gate-skill endpoint, while its Zaphod boundary recorder proves zero verdict, routing, decision-log, and provider-state calls.
  AC-O4 → *One receiver, one explicit acceptance* and test-plan steps 3–4: the protocol trace fixture covers explicit decline, accepted replay, lost reply, exit, and direct-launch count; only explicit decline permits the fallback.
  AC-C1 → *Riskiest unproven mechanism* and test-plan steps 1, 5, and 6: the planned CL drill uses the passed disposable-profile handoff in `foreground-attached-client-profile.md:148-230`, the existing two-tab host pre-spike, and gate-skill receipts. It remains a planned live proof, not a claimed run.
- DONE: Re-run the ideation AC scan and leave no unevidenced acceptance criterion.
  `spacedock status --read v1-gate-review-surface-handoff --stage ideation --ac-scan --json` is the gate-facing verifier; this cycle cites AC-O1, AC-O2, AC-O3, AC-O4, and AC-C1 inside checklist evidence rather than only in summary prose.
- DONE: Keep this as evidence repair only: no implementation, no scope growth, and no additional live harness or 7h work without captain authorization.
  This append changes only report evidence. The approved body, ACs, deferrals, native pre-spike, and held CL drill remain unchanged; no code, profile, 7h, or 4d file was touched.

### Summary

This cycle makes every acceptance criterion auditable from the ideation report. The approved handoff stays intact: Zaphod owns only one receiver-local surface and its lifecycle, and the gate skill retains result validation and routing. The only live evidence remains the existing isolated host pre-spike; the contract-dependent CL proof stays pending.
