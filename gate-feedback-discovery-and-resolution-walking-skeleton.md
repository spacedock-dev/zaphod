---
id: pzw1ctjmej8bjy9t4td91yy8
title: Gate feedback discovery and resolution automation — walking skeleton across the sprint 2/3 seam
status: ideation
source: finding — live session dogfooding, 2026-07-08 (CL left subspace-tui feedback that sat unread with no automated consumer)
started: 2026-07-08T08:48:18Z
completed:
verdict:
score: 0.85
worktree:
issue:
pr:
mod-block:
---

## Problem

Live tonight: CL clicked a gate row, floated `subspace-tui`, left a comment,
quit. The feedback landed correctly in
`dock-toggle-restructures-panes.decisions.jsonl` — but nothing consumed it.
It sat unread until CL explicitly asked "who consumes the response?" and the
FO went and manually `cat`'d the file. There is no automated discovery or
resolution path today; this is the documented, intentional current state,
not a regression:

- `docs/plan-agent-rail.md`'s **Dogfood posture** (line 92-94): "Artifact
  review moves into the fresh zellij session starting now: pre-sprint-2
  gates reviewed there manually via `subspace-tui`; sprint 2 automates the
  discovery."
- **Sprint 2 — "gates for real + review dogfood"** (line 67-76): "Glob
  watching per the config; fold via the subspace binary — never reimplement
  fold... This project's own review gates run through subspace review in the
  fresh session — the rail's first real tenant is zaphod development
  itself." Exit: "a real zaphod dev gate resolved via a rail-surfaced
  review."
- **Sprint 3 — "M2 seam"** (line 78-85): "The subspace gate server writes
  `<log>.addr` beside the log (additive, no model change); rail verdict
  actions — approve / revise / reject / hold — POST to it. Direct log
  append stays forbidden (single-writer flock)." Exit: "a verdict issued
  from the rail lands on the record and unblocks the waiting agent."

Both sprints are prose in a plan document, not filed tasks. This entity
files the work and asks ideation to design it as a **walking skeleton**:
the thinnest possible slice that proves the *entire* loop — glob-watch →
fold → surface in the rail → a rail verdict action → POST to a gate server
→ unblock the waiting/polling agent — closes end-to-end, before either
sprint's full generality (multiple concurrent gates, all four verdict
types, arbitrary glob configs) is built out.

## Proposed approach

Apply the walking-skeleton principle explicitly: ideation should design the
smallest end-to-end path that exercises every layer at least once, not a
polished slice of one layer. Concretely, candidates for what the skeleton
cuts away (ideation should confirm or replace these, not treat them as
fixed):

- **One gate, one verdict.** Prove "approve" unblocking a waiting agent
  before designing revise/reject/hold.
- **Discovery: naive polling + globbing is explicitly acceptable for this
  pass** (CL's direction, 2026-07-08) — periodically glob for
  `*.decisions.jsonl` and re-fold. This accumulates `.jsonl` files over
  time and is not the final mechanism; that's an accepted, named cost for
  the skeleton, not an oversight. Do not let ideation "improve" this into a
  fully event-driven watcher on this pass — the point is proving the loop
  closes, not building the production discovery mechanism. A follow-on
  task replaces polling with "a real consumer" (CL's words) once the loop
  is proven; name that follow-on explicitly rather than solving it here.
- **Fold via the existing subspace binary as a black box** ("never
  reimplement fold," per the plan) — call it, don't rebuild its logic.
- **The `<log>.addr` + POST mechanism (Sprint 3) is the actual unproven
  risk** — a `hold`-style resolution that unblocks a *real* waiting agent,
  not just a UI affordance that writes a file nobody reads. That's the
  piece the walking skeleton must prove first, per this workflow's own
  spike discipline ("name the task's riskiest unproven mechanism... the
  smallest end-to-end check that would invalidate the design listed first
  in the test plan").

Ideation should also determine whether this is one task or should split
into a skeleton task (thin, end-to-end) followed by separate generalization
tasks (multi-gate, all verdict types, full glob config) — do not assume a
single task covers both without stating why.

## Acceptance criteria

Ideation designs these; do not treat the following as fixed, only as the
shape the walking-skeleton principle implies:

**AC-1 — A gate decision log written by `subspace-tui` is discovered
without a human manually checking for it.**
Verified by: an end-to-end demo — leave feedback via `subspace-tui`, and
some automated watcher (even a minimal single-path one) surfaces it,
against an independent baseline (a timer/log showing the watcher noticed
without a human poll).

**AC-2 — A verdict issued from the rail actually unblocks a real waiting
agent**, not just a UI action with no observable effect.
Verified by: a live demo with a genuinely polling/blocked agent (subspace's
own `hold` semantics, per the plan) that resumes only after the rail
verdict POSTs to the gate server.

**AC-3 — The skeleton proves the full loop with the narrowest possible
scope**, explicitly deferring generalization (multi-gate, all verdict
types, arbitrary glob config) to named follow-on work, not silently
building full generality into "AC-1."

## Test plan

Riskiest first, per this workflow's own spike discipline: the Sprint-3
POST-to-unblock mechanism against a real waiting agent is the one unproven
end-to-end claim (does a rail-issued verdict actually resume a blocked
agent, not just look like it does in the UI) — prove that before polishing
discovery/glob-watching, which is comparatively low-risk plumbing.

## Out of scope

Full glob-config generality, all four verdict types (revise/reject/hold
beyond whichever the skeleton needs to prove unblocking), and multi-gate
concurrent review — these are real Sprint 2/3 scope but not the walking
skeleton; ideation should name them as explicit follow-on work rather than
building them now.
