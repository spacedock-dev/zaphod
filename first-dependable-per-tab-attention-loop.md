---
title: First dependable per-tab attention loop
status: backlog
source: outcome-first roadmap Sprint 2, captain direction 2026-07-11
sprint: s2-dependable-per-tab-attention-loop
sprint-lane:
sprint-entry:
score: 1.0
started:
completed:
verdict:
worktree:
issue:
pr:
mod-block:
id: e6djd6wv1rxvj5zq4wqcw2ja
group: walking-skeleton
sprint-readiness: defer
blocked-on: 7h-validation-and-continuity-gate
blocked-reason: await the 7h validation gate and a continuity drill that identifies the smallest missing behavior
---

## Problem

A reliable current-tab rail needs one complete, repeatable attention journey before any broader workspace architecture is justified.

## Required outcome

From normal Zellij work, show one real session and one real pending review in the current tab; focus the session or open the provider-owned review UI; after the provider decision, show truthful updated state without tab hunting.

## Dispatch boundary

Remain backlog until 7h passes and the continuity gate identifies the smallest missing behavior. Do not introduce a hub, managed tab, native launcher, pane adoption, or inline review controls without that evidence.

## Acceptance criteria

Ideation must turn the continuity-gate finding into one smallest end-to-end implementation and one live operator-loop gate.
