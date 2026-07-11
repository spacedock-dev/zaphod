# Zaphod roadmap

This roadmap orders product work by the operator outcome it delivers. The
current per-tab WASM rail is a supported baseline: it already helps an
operator see what is happening in the current tab and act on it. The
[workspace architecture](zaphod-workspace-architecture.md) remains the
long-term design; it does not authorize replacing working behavior without a
demonstrated operator benefit.

`docs/plan-agent-rail.md`, the root prototype documents, and their old sprint
numbers are historical build records. They remain useful evidence, but this
file controls delivery order.

## Delivery rules

- Start each product sprint from one complete operator journey, not from a
  component list.
- Preserve reliable behavior until a replacement proves equal or better value
  in the same operator journey.
- Name the operator failure before replacing a working surface. Architectural
  neatness alone is not a failure.
- Keep review truth and review resolution in the provider. Zaphod may surface
  a review and open the provider UI; it does not render a provider form or
  issue a verdict.
- Run live captain drills only after reproducible offline checks pass.
- Never mutate standing Zellij or tmux configuration during development tests.

## Current product baseline

The shipped rail is reliable for its current tab. It lists terminal panes and
agent state, and its companion process can surface session and review rows.
Session actions focus a bound pane. Review actions open the provider's review
surface. This is the behavior later work must preserve or improve.

The legacy `grout` and rail tasks contain valuable evidence:

- `yb` and `7v` show session ingestion, stale-data handling, and the current
  rail's focus behavior.
- `pz` proves that a provider-owned gate server, rather than a direct log
  append, owns durable resolution and waiter wake-up.

Their implementation boundaries do not become mandatory product architecture.
In particular, do not carry forward `pz`'s rail-issued `approve` action: it
violates the provider-owned review boundary.

## Sprint 1 — trusted test-profile onramp (in flight)

### End value

An operator can exercise real keys in an isolated Zellij profile and trust
that exit, signals, and cleanup leave standing configuration untouched. This
is an enabling exception to the walking-skeleton rule: it makes later product
drills trustworthy without claiming to ship a new attention surface.

### Locked scope and sequence

Sprint 1 has completed staff review. Do not add to or reshape its task bodies,
statuses, worktrees, or order. The First Officer may maintain its delivery
metadata in frontmatter.

Its membership is the query `sprint=s1-trusted-test-profile-onramp`. The
`group` and `sprint-readiness` fields make the release lane visible without
duplicating a mutable task list in this document.

1. **`foreground-attached-client-profile` (`7h`)** is the sole active Sprint 1
   release-path lane. It must pass its foreground-PTY, raw-input, cleanup, and
   global-isolation gate.
2. **`zaphod-native-cli-skeleton` (`bc`)**, **`managed-view-driver-contract`
   (`qb`)**, and **`zellij-managed-identity-feasibility` (`6v`)** remain
   captain-approved but paused after `7h`. They are neither canceled nor
   automatically dispatched.
3. No Sprint 2 product task depends on those paused lanes unless the
   continuity gate below identifies an operator failure that needs the
   managed-workspace path.

### Gate

The `7h` validation gate is Sprint 1's stop point. Passing it authorizes
safe, repeatable live drills; it does not automatically start `bc`, `qb`, or
`6v`.

## Continuity gate — keep, evolve, or replace the per-tab rail

Run this gate after `7h` passes and before dispatching paused foundation work
or a replacement architecture.

### Question

Can the current per-tab rail complete the operator's attention loop in normal
Zellij work, or is there a concrete failure that it cannot correct without a
different boundary?

### Drill

In one real working tab, the operator must be able to:

1. see a live session that needs attention;
2. focus that session's bound pane from the rail;
3. see one pending review;
4. open the provider-owned review UI from the rail;
5. make the decision in that provider; and
6. see the resulting provider state reflected without tab hunting.

Record every missed item, stale state, wrong focus target, foreign-tab change,
or review-launch failure. A successful drill keeps the per-tab rail as the
product baseline. A replacement path requires a named failure and an
equal-or-better cutover drill for this journey.

## Sprint 2 — dependable per-tab attention loop

### End value

From normal Zellij work, an operator sees live session and pending-review
attention in the current tab, focuses the right session, or opens the review
in its provider-owned UI. After the provider resolves the review, the rail
shows truthful updated state. The operator no longer hunts through tabs to
find the next interruption.

### Scope

Sprint 2 is one outcome-owned task plus one operator gate. It may reuse the
current WASM rail and `grout` where they already serve the journey. It may
change their internals only to close a failure exposed by the continuity gate.

The sprint does not require a hub, a managed tab, a native launcher, a generic
provider framework, tmux support, pane adoption, or inline review controls.
Those are possible later responses to measured limits, not prerequisites.

### Task and gate map

| Item | Purpose | Dispatch rule |
| --- | --- | --- |
| **First dependable per-tab attention loop** | Own the full journey: one real session, one real pending review, focus, provider-owned open, and truthful post-decision update. | Prefiled with `sprint-readiness: defer`. Ideate only after `7h` passes and the continuity gate names the smallest missing behavior. |
| **Sprint 2 operator-loop gate** | Reproduce the full live journey with a real session and review provider. | Run after the task's offline checks. A passing gate proves value; it does not authorize unrelated architecture work. |

### Explicit deferrals

- `fp` — the managed-tab controller and guarded keybindings — remains deferred
  until a continuity-gate failure requires managed entry.
- `1s` — explicit pane adoption — is an optional later capability, pending an
  explicit product decision.
- `bc`, `qb`, and `6v` remain paused as described in Sprint 1.
- `j5`, `eh`, `fw`, `4d`, and `m1` remain foreign-tab retrofit or upstream
  research, outside this release path.
- `s6`, `n5`, `hj`, `yb`, `7v`, and `pz` remain evidence or narrowly scoped
  repair candidates until the attention-loop task selects a proven need.

## Sprint 3 — evidence-led expansion

Sprint 3 is not preallocated to a component. Its goal follows the first
failure that remains after Sprint 2's operator gate: for example, reliable
reconnection and stale-state recovery, a second supported tab/workspace, or a
managed-workspace migration that passes the continuity cutover test. Write its
task only after Sprint 2 records that evidence.

## Operational note

The workflow currently has no dispatchable tasks because its two implementation
slots are occupied by active `7h` and stale `4d` state. This roadmap recommends
deferral of `4d` because it is outside the release path; it does not mutate its
state. A captain decision is required before changing that record.
