---
title: Live sessions arrive and lead back to work
status: backlog
source: captain direction 2026-07-11; Sprint 2 outcome shaping
sprint: s2-dependable-per-tab-attention-loop
group: walking-skeleton
sprint-readiness: ready
blocked-on: 7h-validation-gate-before-implementation
blocked-reason: Ideation is captain-approved now. Implementation and the live drill require the passed disposable-profile gate.
score: 1.0
started:
completed:
verdict:
worktree:
issue:
pr:
mod-block:
id: bb3sedraaa53wa7wjp8xf0p7
---

## Problem

A normal Zellij tab does not continuously turn live top-level agent sessions into trustworthy, focusable current-tab attention.

## Required outcome

The current-tab rail continuously shows genuine top-level sessions for that tab, focuses an unambiguous bound pane on click, and removes departed sessions without manual one-shot commands or tab hunting.

## Ideation boundary

Design one profile-scoped subscriber path: initial, SSE data_changed, reconnect, and periodic list refresh; authoritative top-level-session filtering; current-tab binding; stale expiry; and bounded failure behavior. Use yb and hj as evidence, not as unchanged dispatches. Do not add gates, pane adoption, managed tabs, or review controls.
