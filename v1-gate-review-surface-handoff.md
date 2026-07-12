---
title: One v1 review opens and returns cleanly
status: ideation
source: captain direction 2026-07-11; Sprint 2 outcome shaping
sprint: s2-dependable-per-tab-attention-loop
group: walking-skeleton
sprint-readiness: defer
blocked-on: s9-unified-gate-walking-skeleton
blocked-reason: The first global gate-to-resolution slice in s9 absorbs qt’s minimal review-surface behavior; qt is reserved for later refinement.
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

## Staff-review revision (cycle 4 — controlling contract)

Binding staff review returned this record to ideation because its previous
handoff treated Zaphod's bridge as if it were a gate contract. This revision
keeps one visible review useful, but puts selection, launch, result, and retry
authority back in the gate skill. The cycle-3 proposal below is retained only
as historical evidence; this section controls implementation.

### Dependencies before implementation

No implementation of this task starts until both inputs have passed their own
acceptance gates:

- **`s9` accepted gate-skill snapshot contract.** The configured gate skill
  must publish one complete global open-gate snapshot with a provider-owned
  opaque `gate_key`. It alone decides whether a key is still open and turns a
  user selection into one opaque selected-gate record. A global row is not a
  tab-origin claim and does not contain a brief path, log path, native tab ID,
  or a reviewer command.
- **`bb` accepted direct-entry receiver handoff.**
  `scripts/zellij-new-tab.sh` must create the fresh managed tab, verify its
  exact resident rail through native state, and start the private
  `zaphod subscribe` process. Only that live, verified rail may advertise a
  short-lived receiver. `Alt Shift z` stays fresh-tab-only: under Zellij it
  cannot start the sidecar through `Run` without opening a visible helper pane.
  The native tab/pane values remain inside bb's target-liveness guard; they are
  never a durable gate identity or data that the gate skill has to interpret.

These are contract dependencies, not invitations to revive 7h, 4d, leases,
controllers, or a public `grout` command. If bb cannot provide its direct-script
receiver without changing that user journey, qt is blocked: it must not paper
over the incompatibility with an `Alt Shift z` helper pane or a guessed target.
If either contract changes at its gate, qt returns to ideation rather than
guessing a substitute.

### Problem

The legacy gate-row click derives a `subspace-tui` command from a decision-log
path, opens it directly, and discards the pane it created. It cannot tie an
open review to one selected pending gate, prevent a retry from making a second
surface, or distinguish a closed pane from a review decision.

That violates Review & Gate v1: workflow tooling owns the immutable Briefing,
authorized approver, reviewer launch authority, result validation, and
routing. Zaphod may make a trusted reviewer visible beside work; it must not
derive or interpret a review decision.

### Required outcome

**Trigger:** the operator selects one global pending gate from the accepted s9
projection while an accepted bb receiver is live in the selected managed rail.
**Visible result:** exactly one provider-owned reviewer opens beside that rail;
the other rail remains unchanged. When the reviewer exits, only that command
pane is removed. The gate skill receives and validates the v1 result through
its own path, routes the workflow, and a later s9 snapshot updates the row.

If no live receiver is known *before* delivery, the gate skill may use its own
ordinary review surface. That fallback is a gate-skill choice, not a legacy
Zaphod float. A selected receiver is a placement choice only: every v1 gate is
global until a later, separately proved origin contract exists.

~~~text
s9 snapshot -> gate skill selects opaque gate -> live bb receiver
     ^                                                |
     |<-- later provider snapshot <- skill validates -+-> Zaphod places one pane
                                             reviewer result      (no decision)
~~~

### Ownership boundary

- The **gate skill** owns `selected_gate`, the Briefing, authorized approver,
  configured reviewer launch, result channel, retry/fallback policy, result
  validation, and workflow routing. It validates that the selected gate remains
  open before offering it.
- The **reviewer app** verifies and renders the Briefing, attributes entries,
  and writes its portable review result. It does not obtain its authority from
  Zaphod.
- **Zaphod** accepts one live receiver handoff, opens one already-authorized
  command pane beside that receiver, and removes only that exact pane on exit.
  It sees no decision, annotations, actor identity, routing data, review-log
  path, or provider-state mutation request.

The native `open_command_pane_floating_near_plugin` API proves same-tab
placement only. It is not a gate protocol, a selection mechanism, a result
channel, or evidence that an offer reached the intended rail.

### Proposed approach

#### One skill-owned selected-gate offer

The gate skill allocates one opaque selected-gate ID and keeps it stable across
status checks and delivery retries. A handoff contains only that opaque
selection, a receiver capability, and a gate-skill-authorized launch ticket:

~~~text
OpenSelectedGateV1 {
  selected_gate: GateSkillOpaqueId,
  receiver: VolatileZaphodReceiver,
  launch_ticket: GateSkillAuthorizedV1Launch
}

SurfaceReceiptV1 {
  selected_gate: GateSkillOpaqueId,
  state: opening | opened | not_opened | unavailable,
  surface_ref?: ZaphodOpaqueSurfaceRef
}
~~~

`launch_ticket` is produced and validated by the configured gate-skill
runner—not reconstructed from a rail row, title, CWD, log path, or command
string. Zaphod may invoke the resulting fixed, trusted review runner but never
chooses an executable or parses a result. `surface_ref` is opaque to the skill;
the native terminal pane ID stays inside Zaphod's live receiver.

The bb sidecar advertises a receiver only while its exact rail target remains
live. Multiple rails may have the same WASM URL, so a receiver capability must
be unique to the live instance; URL, tab name, CWD, raw tab ID, pane ID, and
active client are not receiver selection. The private transport may reach more
than one resident plugin, but every nonmatching receiver must ignore the offer.

#### Delivery, failure, and retry

The gate skill, rather than Zaphod, owns this state machine:

1. With no currently live receiver, the skill records that fact and may make
   one direct provider-owned launch. It never asks Zaphod to guess a tab.
2. With a live receiver, the skill offers the same selected-gate ID. The
   receiver reserves `opening` before scheduling host work. A duplicate offer
   returns the existing state and cannot create another pane.
3. `opened` means Zaphod has an exact returned terminal pane ID. `not_opened`
   or `unavailable` means the receiver can prove that no pane was launched;
   only then may the skill take its one direct fallback.
4. A lost reply, `opening`, or `opened` is indeterminate or already accepted:
   the skill queries the same selected-gate status and does not issue a fresh
   selection, a second launch, or a direct fallback. If status cannot settle,
   it reports the uncertainty through its normal gate handling rather than
   assuming a decision or a missing pane.
5. A manual close or exit is a lifecycle notice, not a result. With no valid
   v1 result, only the gate skill may decide whether a later user action makes
   a fresh selected-gate attempt; Zaphod never relaunches it automatically.

This gives a caller a clear retry path without a pool, controller, durable
binding, or hidden prewarmed pane.

#### Exact visible-pane lifecycle

For each live receiver, Zaphod keeps one in-memory record keyed by the opaque
selected-gate ID: `Opening`, then `Open { surface_ref, terminal_pane_id,
context }`, then removal. It is not recovered after a plugin restart and is
never reconstructed by scanning tabs, titles, logs, layouts, or paths.

The pipe/receiver path records `Opening` only; the host call occurs in a normal
`update()` path, never in `pipe()` or `load()`. It calls
`open_command_pane_floating_near_plugin` with a context containing only the
opaque selection and surface reference. Only `Some(PaneId::Terminal(_))`
produces `opened`; `None`, permission refusal, or receiver loss before that
produces the explicit no-surface state.

Zaphod subscribes to `CommandPaneExited` and `PaneClosed`. On an exit, it
calls `close_terminal_pane` only when the terminal ID *and* opaque context
match its own open record. The matching `PaneClosed` removes that record.
Wrong IDs, wrong context, stale events, a different rail, and an ordinary user
close do nothing beyond their matching lifecycle notice. Zaphod never closes a
foreign pane, relaunches a reviewer, or routes a gate.

#### Result authority and existing seams

The reviewer result travels directly to the gate skill's v1 result handling.
The skill verifies the Briefing and authorized approver, interprets approve,
revise, or hold, and performs its own route. A later accepted s9 snapshot is
the sole source of a changed rail row. Zaphod writes no review log, verdict,
decision log, or provider state.

qt extends a pure ephemeral surface-record seam plus `Sidebar::update` and
the private bb receiver handoff. It does not reuse
`ClickAction::FloatGate`, `brief_path_for_log`, or the legacy direct
`open_command_pane_floating` path for v1 rows. The old path remains historical
behavior until separately retired; it is not a fallback after an accepted v1
offer.

### Riskiest unproven mechanism

The first risk is exact live receiver delivery and receipt—not native floating
placement. The smallest invalidator starts the standard isolated tmux-hosted
Zellij profile with two rails using the same WASM URL. bb's fixture advertises
only receiver A; receiver B is live but has a different capability. An s9
fixture presents global gate `alpha`; the gate-skill fixture selects `alpha`
and delivers the same offer twice, including one lost-reply/status-query case.

Native pane inventory must show one reviewer terminal beside A and none beside
B. The fixture must observe one stable `opened` result, no direct fallback,
and on runner exit only that terminal disappears. Separately, a controlled
valid v1 result reaches the gate-skill fixture once; its route record changes
while a Zaphod decision recorder remains empty. A pipe that reaches B, a second
pane, receipt ambiguity that falls back, unmatched cleanup, or a Zaphod
decision call invalidates the design. The profile uses temporary tmux/Zellij
roots and ordinary cleanup—never 7h or a custom PTY harness.

### Acceptance criteria

#### Offline

**AC-O1 — One selected gate reaches only its live receiver.** Given accepted
s9 fixture snapshot `alpha` and bb receiver fixtures A and B, replaying the
same selected-gate offer opens one terminal beside A, returns one stable opaque
surface reference, and opens none beside B. A stale, wrong, or absent receiver
opens none.

Verified by: gate-skill offer and Rust receiver/host fakes consume s9 and bb
fixtures, measure one A launch, zero B launches, and stable replay receipts.
Expected gate keys and receiver capabilities come from those independent
fixtures, not from Zaphod-authored values.

**AC-O2 — Retry and fallback never duplicate a review.** `unavailable` and
explicit `not_opened` before a pane exists permit exactly one gate-skill direct
launch. A replay, `opening`, `opened`, lost receipt, status timeout, manual
close, or exit permits zero direct fallback and zero second Zaphod launch.

Verified by: a gate-skill protocol fake drives each trace against direct-launch
and Zaphod-host recorders, asserting the externally specified launch counts.

**AC-O3 — Zaphod cleans up only its own surface and has no decision authority.**
A matching `CommandPaneExited`/`PaneClosed` pair closes and removes its exact
terminal once. Wrong terminal IDs, contexts, receiver capabilities, and foreign
closes make no host close call. A valid result produces one gate-skill route;
an exit without a valid result produces none; Zaphod records zero decision,
review-log, or provider-state calls.

Verified by: recorded native event fixtures and a v1 Briefing/authorized-
approver result fixture drive fake Zellij and gate-skill sinks with exact call
counts.

**AC-O4 — The real receiver-to-pane path is isolated and fail-closed.** In a
temporary-root tmux-hosted Zellij server, the accepted bb
`scripts/zellij-new-tab.sh` entry brings up two rails; accepted s9 data selects
`alpha`; the real private receiver path opens and removes one fixture reviewer
only beside A. A missing receipt, permission refusal, target loss, or B-token
offer leaves no additional pane and no standing configuration change.

Verified by: native `list-panes` and visible-screen captures before, during,
and after the fixture runner, plus temporary-root and tmux cleanup checks.

#### Captain-live

**AC-C1 — One review returns to the operator loop without tab hunting.** The
operator sees an open global s9 gate in a bb-initialized rail, selects it, and
gets one real v1 reviewer beside that rail while a second rail remains
unchanged. After a real authorized approve, revise, or hold, the gate skill
routes it and a later provider snapshot truthfully changes the rail. No manual
pipe, log-path command, or Zaphod verdict is involved.

Verified by: a two-tab captain drill retains native before/during/after pane
inventory, the gate-skill selected-gate/receipt/result evidence, and the later
provider snapshot.

### Test plan

1. Prove AC-O4's two-rail receiver/receipt invalidator first. Do not treat the
   near-plugin API as a receipt protocol or replace this with a KDL read,
   custom PTY, or hand-issued float.
2. Add pure selected-gate and receiver tests for stale/wrong capability,
   duplicate delivery, opening, explicit no-surface, lost receipt, status
   query, target loss, manual close, and no automatic relaunch.
3. Add exact `CommandPaneExited`/`PaneClosed` context tests and the gate-skill
   result-boundary tests. Keep result validation and routing outside Rust.
4. Run focused Rust and gate-skill suites, the isolated tmux smoke packet, and
   `git diff --check`. Failed receiver delivery is a visible no-surface result,
   never an invitation to use the legacy click path.
5. Run AC-C1 only after the offline packet passes and both s9 and bb acceptance
   gates have passed. Use the real v1 reviewer, not `--gate-review`'s legacy
   stdout fold.

### Documentation change

When this ships, replace README's legacy gate-float wording with: “A selected
Review & Gate v1 item is opened by its gate skill. When a live Zaphod receiver
is available, Zaphod places one provider-authorized reviewer beside that rail
and cleans up only that pane. The gate skill validates and routes results;
Zaphod never interprets a review decision.” Clarify that `RunCommands` permits
placement only, not row-derived command execution or a rail-owned verdict.

### Out of scope

Tab-origin inference; durable gate/tab bindings; raw native tab or pane IDs as
gate identity; pane adoption; controllers; leases; 7h; 4d; custom PTYs; a
public `grout` command; a public Zaphod gate command; a broad daemon or review
pool; hidden prewarming; automatic relaunch; source discovery; log-path or
stdout scraping; arbitrary row-supplied commands; session ingestion; and any
Zaphod interpretation of approve, revise, hold, rejection, or workflow state.
The legacy direct float is not a v1 fallback and is not reused by this slice.

## Historical cycle-3 proposal (superseded)

The material below records the previous design for audit only. It does not
define implementation prerequisites, wire semantics, or acceptance criteria.

### Problem

The legacy gate-row click derives a subspace-tui command from a log path,
opens it directly, and discards the returned pane ID. It cannot prevent a
duplicate review, clean up only the pane it opened, or distinguish a reviewer
exit from a gate decision.

That boundary is wrong for Review & Gate v1. The v1 contract assigns workflow
position, approver authority, result validation, and routing to workflow
tooling. The reviewer app owns its review log and result. Zaphod may place a
reviewer beside work; it must not decide what approve, revise, or hold means.

### Required outcome

For one pending Review & Gate v1 briefing, the gate skill owns one review
attempt. When the operator has selected a live Zaphod rail, the skill may ask
that rail to open one visible reviewer in its own tab. When no receiver is
available before the request, the skill opens its own direct review surface.
After the reviewer exits, the gate skill validates the v1 result and routes the
workflow; the rail later reflects provider truth.

The first path is global-first. A selected rail is a transient placement choice,
not proof that the gate originated in its tab. Later origin projection may make
placement smarter, but it is not a prerequisite for this loop.

### V1 boundary

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

### Proposed approach

#### One skill-owned handoff

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

#### Global receiver and fallback

The receiver exists only while its rail instance is live. It denotes “place
this one surface beside this rail,” not a canonical workspace, managed-tab
record, tab origin, or durable gate association. A global gate may therefore
open beside the rail the operator selected.

The gate skill may direct-float once only when it knows that no receiver was
available, or receives an explicit decline before Zaphod launches anything.
An opening, accepted, or indeterminate handoff never falls back. This avoids
duplicate surfaces without adding a controller or a pool.

#### Exact-pane lifecycle

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

#### Result authority stays outside Zaphod

The configured runner returns the Review & Gate v1 result to the gate skill.
The skill verifies the Briefing and authorized approver, then interprets
approve, revise, or hold and advances, revises, or parks workflow work. Zaphod
neither reads the result nor writes a review log, verdict, decision log, or
provider state. It may report that its surface exited; provider updates are the
only source of a changed rail row.

#### Existing seams

The implementation extends a pure ReviewSurfaceRegistry plus the existing
Sidebar::update seam. It uses Zellij's
open_command_pane_floating_near_plugin, CommandPaneExited, and PaneClosed
interfaces. The existing legacy ClickAction::FloatGate, brief_path_for_log, and
direct open_command_pane_floating path remain unchanged; they are neither a v1
result route nor a v1 fallback after acceptance.

### Riskiest unproven mechanism

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

### Acceptance criteria

#### Offline

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

#### Captain-live

**AC-C1 — One real review opens beside selected work and returns without tab
hunting.** CL selects a pending global gate from rail A while rail B remains
foreign. One v1 reviewer appears beside A. After CL submits a v1 decision, the
gate skill routes it and the later provider projection updates the rail; both
tabs retain their ordinary panes.

Verified by: the isolated two-tab drill captures native list-panes before,
during, and after; the gate skill's receipt and validated result; and the
provider's resulting gate projection. It records no second surface and no
Zaphod verdict.

### Test plan

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

### Documentation change

When this path ships, replace the README's gate-float wording with: “For an
accepted Review & Gate v1 handoff, Zaphod opens one provider-owned reviewer
beside the selected rail and closes only that surface after it exits. The gate
skill validates results and routes decisions; Zaphod never interprets a review
decision.” Update the permission note to say RunCommands permits the surface
launch, not a rail-owned verdict.

### Out of scope

Canonical tab origin, a managed-view controller, binding records, pane
adoption, leases, a 7h profile pass, custom PTY control, a broad daemon,
grout subscribe, session ingestion, and an origin-based gate projection. So
are a public CLI, arbitrary commands from gate rows, log-path-derived launches,
stdout or decision-log scraping, hidden prewarming, a review pool, automatic
relaunch, and any Zaphod interpretation of approve, revise, hold, or a
workflow rejection. The legacy direct float remains untouched in this slice.

### Historical stage reports

The reports below preserve the earlier, richer proposal for audit. The body
above controls this task; it deliberately removes its speculative durable
receiver and result-channel design.

#### Stage Report: ideation

- DONE: Define the accepted review-surface handoff and result contract so gate skill routing stays separate from Zaphod surface ownership.
  The record requires a one-receiver offer, post-open acceptance receipt, opaque gate-skill result capability, and lifecycle-only Zaphod notices.
- DONE: De-risk same-tab surface correlation, exit/close handling, and no-duplicate/no-foreign-tab behavior with the smallest live spike.
  An isolated two-tab Zellij 0.44.3 run placed and removed one fixture float only in origin; two identical launches produced two floats, proving the registry must enforce idempotence.
- DONE: Keep the first slice visible and simple: fallback only before acceptance, no hidden prewarm, and no Zaphod decision semantics.
  Explicit decline is the sole fallback permission; accepted and indeterminate handoffs never fall back, and result interpretation remains entirely with the gate skill.

##### Summary

The earlier v1 path treated the surface handoff as a controller. Its useful
remnants are exact-pane lifecycle and no-duplicate handling; its durable
receiver and result-channel requirements are superseded.

#### Stage Report: ideation (cycle 2)

- DONE: Retain the approved v1 handoff design and add explicit Stage Report evidence mappings for every acceptance criterion, especially AC-O2 through AC-O4.
  This report is superseded by cycle 3's global-first body and acceptance criteria.
- DONE: Re-run the ideation AC scan and leave no unevidenced acceptance criterion.
  This report is superseded by cycle 3's new AC scan.
- DONE: Keep this as evidence repair only: no implementation, no scope growth, and no additional live harness or 7h work without captain authorization.
  This remains true; cycle 3 changes only this task record.

##### Summary

Cycle 2 recorded evidence for the prior design. Cycle 3 replaces that design
with the gate-skill-owned v1 boundary above.

#### Stage Report: ideation (cycle 3)

- DONE: Reduce review opening to one visible, skill-owned handoff.
  The gate skill now owns the Briefing, runner, result, validation, and route; Zaphod owns only accepted placement and exact pane lifecycle.
- DONE: Remove 7h and speculative durable identity prerequisites.
  The body starts global-first and excludes a canonical origin, controller, lease, custom PTY, profile pass, daemon, and binding record.
- DONE: Name the smallest exact-pane lifecycle proof and fallback boundary.
  The first test is an isolated tmux-hosted two-tab native run: duplicate delivery yields one A-tab pane, zero B-tab panes, exact cleanup, one gate-skill result, and no fallback after acceptance.
- DONE: Map every acceptance criterion to an external fixture or live observation.
  AC-O1 uses the gate-skill offer fixture and fake host counts; AC-O2 uses recorded native event fixtures; AC-O3 uses the external Briefing, approver, and gate-skill route fake; AC-O4 uses the direct-launch fake; AC-C1 uses native pane inventory plus the gate skill's validated result and provider projection.

##### Summary

This re-ideation aligns the task with Review & Gate v1. It preserves only the
small registry required to prevent duplicate or foreign floats, and it gives
all gate semantics back to the gate skill. No product code, 7h, 4d, or other
entity changed.

## Stage Report: ideation (cycle 4)

- DONE: Define a gate-skill-owned selected-gate and live-receiver handoff.
  The controlling revision names `OpenSelectedGateV1`, keeps the selected gate,
  Briefing, launch ticket, result, and route in the skill, and makes bb's live
  direct-script receiver the only Zaphod placement input.
- DONE: Specify failure, retry, and exact-pane lifecycle without decision authority.
  The delivery state table permits a direct fallback only after explicit
  no-surface evidence; replay, loss, opening, opening success, exit, and close
  cannot duplicate a review, while matching exit/context cleanup is Zaphod-only.
- DONE: Align with the accepted global snapshot boundary rather than legacy click paths.
  qt now waits for accepted s9 and bb contracts, treats s9 rows as global
  provider truth, excludes raw locator identity, and bans `FloatGate` and
  log-path inversion from its v1 path.

### Summary

The revised task is a narrow visible-review handoff, not a Zaphod gate system.
It explicitly depends on s9's selected-gate snapshot contract and bb's
direct-script live-receiver contract; `Alt Shift z` remains fresh-tab-only
because a Zellij `Run` sidecar would visibly disturb the user journey. No
product code or parked 7h/4d work changed.
