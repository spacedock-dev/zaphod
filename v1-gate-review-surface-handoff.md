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

The legacy gate-row click derives a subspace-tui command from a log path,
opens it directly, and discards the returned pane ID. It cannot prevent a
duplicate review, clean up only the pane it opened, or distinguish a reviewer
exit from a gate decision.

That boundary is wrong for Review & Gate v1. The v1 contract assigns workflow
position, approver authority, result validation, and routing to workflow
tooling. The reviewer app owns its review log and result. Zaphod may place a
reviewer beside work; it must not decide what approve, revise, or hold means.

## Required outcome

For one pending Review & Gate v1 briefing, the gate skill owns one review
attempt. When the operator has selected a live Zaphod rail, the skill may ask
that rail to open one visible reviewer in its own tab. When no receiver is
available before the request, the skill opens its own direct review surface.
After the reviewer exits, the gate skill validates the v1 result and routes the
workflow; the rail later reflects provider truth.

The first path is global-first. A selected rail is a transient placement choice,
not proof that the gate originated in its tab. Later origin projection may make
placement smarter, but it is not a prerequisite for this loop.

## V1 boundary

- Review & Gate v1 uses approve, revise, and hold. Reject is a workflow action
  outside the review contract. The legacy --gate-review fold is not v1 workflow
  truth.
- The gate skill owns the immutable Briefing, authorized approver, reviewer
  launch, result delivery, validation, and routing. It carries those inputs;
  Zaphod does not reconstruct them from a row or log path.
- The reviewer app verifies and renders the Briefing, stamps attribution, and
  owns the portable review log. Its result reaches the gate skill's result
  channel, not Zaphod's stdout or a Zaphod parser.
- Zaphod owns only one volatile placement request and the exact command-pane
  lifecycle it created. It receives no decision, annotations, actor identity,
  routing context, or provider-state mutation request.

## Proposed approach

### One skill-owned handoff

The gate skill creates one opaque handoff_id for a review attempt and keeps it
stable across a delivery retry. It holds the v1 Briefing, trusted reviewer
launch reference, and result channel. A live rail contributes only an opaque,
short-lived receiver token.

~~~text
OpenReviewSurfaceV1 {
  handoff_id: GateSkillOpaqueId,
  receiver: VolatileRailReceiver,
  launch: GateSkillTrustedV1Launch
}

ReviewSurfaceReceiptV1 {
  handoff_id: GateSkillOpaqueId,
  status: opening | accepted | declined,
  surface_id?: ZaphodRuntimeId
}
~~~

Launch is a gate-skill-validated v1 reviewer invocation, not a shell string,
row field, log-path inversion, or decision payload. The initial bridge may
recognize one configured review runner; it does not create a general command
executor. Surface_id is opaque to the gate skill and never exposes a native
pane ID or tab ID.

The gate skill invokes a one-shot surface bridge for this offer; the bridge
returns one receipt and exits. It is not grout, a subscription, a daemon, or a
binding store. The gate skill may continue while its provider waits for the
reviewer result, because result consumption remains outside Zaphod.

The rail returns opening while its reserved handoff awaits the host call and
accepts only after Zellij returns Some(PaneId::Terminal(_)) from
open_command_pane_floating_near_plugin. A failed or unavailable receiver
returns an explicit decline without opening a pane. A missing receipt is
indeterminate: the gate skill records the uncertainty or asks for the same
handoff's status; it does not start a second review.

### Global receiver and fallback

The receiver exists only while its rail instance is live. It denotes “place
this one surface beside this rail,” not a canonical workspace, managed-tab
record, tab origin, or durable gate association. A global gate may therefore
open beside the rail the operator selected.

The gate skill may direct-float once only when it knows that no receiver was
available, or receives an explicit decline before Zaphod launches anything.
An opening, accepted, or indeterminate handoff never falls back. This avoids
duplicate surfaces without adding a controller or a pool.

### Exact-pane lifecycle

Zaphod keeps a small in-memory ReviewSurfaceRegistry:

~~~text
handoff_id -> Opening | Open { surface_id, terminal_pane_id } | Closed
~~~

It reserves Opening before scheduling the host call. A duplicate offer in
Opening returns opening; one in Open returns the existing accepted receipt.
Neither launches a second pane. The actual
open_command_pane_floating_near_plugin call runs from a normal update path,
never from pipe() or load(). Its context carries the opaque handoff and surface
IDs.

Zaphod subscribes to CommandPaneExited and PaneClosed. A CommandPaneExited
event closes a terminal only when its terminal ID and context match the registry
entry. A matching PaneClosed removes that entry. Unknown, stale, or foreign
events do nothing. A manual close is lifecycle evidence only; it never
relaunches a reviewer or changes a gate. The registry does not recover after a
plugin restart by scanning tabs, titles, logs, or paths; the gate skill starts a
fresh handoff if needed.

### Result authority stays outside Zaphod

The configured runner returns the Review & Gate v1 result to the gate skill.
The skill verifies the Briefing and authorized approver, then interprets
approve, revise, or hold and advances, revises, or parks workflow work. Zaphod
neither reads the result nor writes a review log, verdict, decision log, or
provider state. It may report that its surface exited; provider updates are the
only source of a changed rail row.

### Existing seams

The implementation extends a pure ReviewSurfaceRegistry plus the existing
Sidebar::update seam. It uses Zellij's
open_command_pane_floating_near_plugin, CommandPaneExited, and PaneClosed
interfaces. The existing legacy ClickAction::FloatGate, brief_path_for_log, and
direct open_command_pane_floating path remain unchanged; they are neither a v1
result route nor a v1 fallback after acceptance.

## Riskiest unproven mechanism

The unproven mechanism is not review semantics; it is delivery of one
gate-skill handoff to one live rail, followed by exact native cleanup. The
smallest invalidator is a tmux-hosted, isolated two-tab Zellij run using the
same temporary config, data, socket, and cleanup pattern as the shipped smoke;
it uses no custom PTY, lease, profile driver, or standing configuration.

The fixture gives rail A one volatile receiver and leaves rail B foreign. It
delivers the same handoff twice to A. Its trusted fixture runner waits long
enough to be observed, writes one valid v1-shaped result to the gate skill's
fixture sink, and exits. Native list-panes must show one reviewer terminal in
A and none in B; the fake gate skill must observe one accepted receipt and no
fallback. On exit, only that terminal disappears. The fixture gate skill then
validates and routes its one result, while a Zaphod decision recorder remains
empty. Any second pane, foreign pane, fallback after acceptance, unmatched
close, or Zaphod decision call invalidates the design.

## Acceptance criteria

### Offline

**AC-O1 — Replayed handoffs create one surface in the selected rail's tab.**
Two deliveries of the same external v1 offer produce one host launch and one
stable opaque surface ID. A second receiver receives no launch.

Verified by: a registry-and-driver test with an external gate-skill offer
fixture and fake Zellij host measures launch count 1, replay receipt count 1,
and foreign launch count 0.

**AC-O2 — Zaphod cleans up only its accepted terminal.** A matching exit
context closes its tracked terminal once. A wrong terminal ID, wrong context,
or unrelated close produces no host close call.

Verified by: recorded Zellij event fixtures, whose IDs and contexts come from
the launch fixture, drive a fake host and measure its exact close calls.

**AC-O3 — The gate skill alone validates and routes a v1 result.** One valid
fixture result reaches the gate skill once and produces its expected route.
Zaphod produces zero workflow, decision-log, or provider-state calls. An exit
without a valid result changes no workflow state.

Verified by: a gate-skill fake validates the v1 result against an external
Briefing and approver fixture, records one route, and a Zaphod boundary fake
records zero decision calls.

**AC-O4 — Fallback occurs only before known acceptance.** A missing receiver or
explicit decline produces one skill-owned direct launch. Accepted, replayed,
opening, exited, and indeterminate handoffs produce zero direct launches.

Verified by: a gate-skill protocol test drives each trace against an external
direct-launch fake and measures its invocation count.

### Captain-live

**AC-C1 — One real review opens beside selected work and returns without tab
hunting.** CL selects a pending global gate from rail A while rail B remains
foreign. One v1 reviewer appears beside A. After CL submits a v1 decision, the
gate skill routes it and the later provider projection updates the rail; both
tabs retain their ordinary panes.

Verified by: the isolated two-tab drill captures native list-panes before,
during, and after; the gate skill's receipt and validated result; and the
provider's resulting gate projection. It records no second surface and no
Zaphod verdict.

## Test plan

1. Run the two-tab native invalidator first with a fixture gate skill and
   runner. Inspect live panes and the gate-skill route record. Do not replace
   it with a KDL read, custom PTY, or a manually asserted transcript.
2. Add pure registry tests for reservation, replay during opening and open,
   explicit decline, indeterminate receipt loss, matching and mismatching exit
   contexts, manual close, and fresh IDs after closure.
3. Add gate-skill/Zaphod protocol tests for direct-fallback boundaries and a
   v1 result that the gate skill, not Zaphod, validates and routes.
4. Run focused Rust tests and the isolated tmux smoke pattern. A missing
   receiver must follow the explicit fallback trace; an unavailable runner or
   result channel must fail loudly.
5. Run AC-C1 only after the automated packet passes. CL's real decision uses
   the v1 reviewer and must not be substituted with the legacy direct float.

## Documentation change

When this path ships, replace the README's gate-float wording with: “For an
accepted Review & Gate v1 handoff, Zaphod opens one provider-owned reviewer
beside the selected rail and closes only that surface after it exits. The gate
skill validates results and routes decisions; Zaphod never interprets a review
decision.” Update the permission note to say RunCommands permits the surface
launch, not a rail-owned verdict.

## Out of scope

Canonical tab origin, a managed-view controller, binding records, pane
adoption, leases, a 7h profile pass, custom PTY control, a broad daemon,
grout subscribe, session ingestion, and an origin-based gate projection. So
are a public CLI, arbitrary commands from gate rows, log-path-derived launches,
stdout or decision-log scraping, hidden prewarming, a review pool, automatic
relaunch, and any Zaphod interpretation of approve, revise, hold, or a
workflow rejection. The legacy direct float remains untouched in this slice.

## Superseded stage reports

The reports below preserve the earlier, richer proposal for audit. The body
above controls this task; it deliberately removes its speculative durable
receiver and result-channel design.

## Stage Report: ideation

- DONE: Define the accepted review-surface handoff and result contract so gate skill routing stays separate from Zaphod surface ownership.
  The record requires a one-receiver offer, post-open acceptance receipt, opaque gate-skill result capability, and lifecycle-only Zaphod notices.
- DONE: De-risk same-tab surface correlation, exit/close handling, and no-duplicate/no-foreign-tab behavior with the smallest live spike.
  An isolated two-tab Zellij 0.44.3 run placed and removed one fixture float only in origin; two identical launches produced two floats, proving the registry must enforce idempotence.
- DONE: Keep the first slice visible and simple: fallback only before acceptance, no hidden prewarm, and no Zaphod decision semantics.
  Explicit decline is the sole fallback permission; accepted and indeterminate handoffs never fall back, and result interpretation remains entirely with the gate skill.

### Summary

The earlier v1 path treated the surface handoff as a controller. Its useful
remnants are exact-pane lifecycle and no-duplicate handling; its durable
receiver and result-channel requirements are superseded.

## Stage Report: ideation (cycle 2)

- DONE: Retain the approved v1 handoff design and add explicit Stage Report evidence mappings for every acceptance criterion, especially AC-O2 through AC-O4.
  This report is superseded by cycle 3's global-first body and acceptance criteria.
- DONE: Re-run the ideation AC scan and leave no unevidenced acceptance criterion.
  This report is superseded by cycle 3's new AC scan.
- DONE: Keep this as evidence repair only: no implementation, no scope growth, and no additional live harness or 7h work without captain authorization.
  This remains true; cycle 3 changes only this task record.

### Summary

Cycle 2 recorded evidence for the prior design. Cycle 3 replaces that design
with the gate-skill-owned v1 boundary above.

## Stage Report: ideation (cycle 3)

- DONE: Reduce review opening to one visible, skill-owned handoff.
  The gate skill now owns the Briefing, runner, result, validation, and route; Zaphod owns only accepted placement and exact pane lifecycle.
- DONE: Remove 7h and speculative durable identity prerequisites.
  The body starts global-first and excludes a canonical origin, controller, lease, custom PTY, profile pass, daemon, and binding record.
- DONE: Name the smallest exact-pane lifecycle proof and fallback boundary.
  The first test is an isolated tmux-hosted two-tab native run: duplicate delivery yields one A-tab pane, zero B-tab panes, exact cleanup, one gate-skill result, and no fallback after acceptance.
- DONE: Map every acceptance criterion to an external fixture or live observation.
  AC-O1 uses the gate-skill offer fixture and fake host counts; AC-O2 uses recorded native event fixtures; AC-O3 uses the external Briefing, approver, and gate-skill route fake; AC-O4 uses the direct-launch fake; AC-C1 uses native pane inventory plus the gate skill's validated result and provider projection.

### Summary

This re-ideation aligns the task with Review & Gate v1. It preserves only the
small registry required to prevent duplicate or foreign floats, and it gives
all gate semantics back to the gate skill. No product code, 7h, 4d, or other
entity changed.
