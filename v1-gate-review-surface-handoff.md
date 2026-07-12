---
title: One v1 review opens and returns cleanly
status: ideation
source: captain direction 2026-07-11; Sprint 2 outcome shaping
sprint: s2-dependable-per-tab-attention-loop
group: walking-skeleton
sprint-readiness: ready
blocked-on: v1-review-surface-contract-pending-gate-projection-and-7h-validation-before-implementation
blocked-reason: Ideation is captain-approved now. Implementation needs the gate skill handoff contract, a real pending-gate projection, and the passed disposable-profile gate.
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
