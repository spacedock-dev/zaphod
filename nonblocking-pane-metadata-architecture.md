---
id: 91f2dxkn3v7fe1174ayj48j5
title: Remove synchronous pane metadata calls from the plugin hot path
status: ideation
source: live nautical-cuckoo congestion diagnosis 2026-07-14
sprint: s1-managed-tab-safety
group: architecture-hardening
sprint-readiness: ready
started: 2026-07-14T11:26:05Z
completed:
verdict:
score: 0.98
worktree:
issue:
pr:
mod-block:
---

## Problem

The WASM event/timer loop synchronously enriches terminal rows through pane command, CWD, and scrollback host calls. Even after the five-second scrollback hotfix, any slow host call can couple sidebar refresh to Zellij's shared runtime queues and make unrelated input or layout actions stall.

## Required outcome

Make the plugin render from bounded cached/event-fed state only. Slow pane enrichment must run outside the WASM hot path with explicit deadlines, limited concurrency, cache/backoff, cancellation, and stale-state presentation. Exact pane identity remains authoritative; unavailable enrichment renders degraded or unbound rather than guessing. Add a congestion regression that exercises a slow/non-shell pane while repeated pane/tab actions remain prompt and do not burst later.

## Out of scope

The exact-plugin close/rebuild/reload operator workflow is tracked by `managed-main-wasm-reload-loop`.

### Feedback Cycles

#### Inherited proof requirements from tactical hotfix 44 — 2026-07-14

- Reuse task 44's structured refresh authority: exact event, plugin, refresh
  ID, and numeric `pane_ids`; do not regress to whole-screen or broad log
  matching.
- Action-preservation evidence must compare the complete relevant terminal
  lifecycle state, including exited, suppressed, floating, and selectable
  fields, with adjacent wrong-pane and wrong-field attacks.
- The congestion proof must hold deterministic slow refresh work in flight
  from before key delivery through native post-action observation. Releasing
  the barrier before observation must fail the test.
- These requirements strengthen the permanent nonblocking architecture; they
  do not expand task 44 beyond removal of periodic scrollback.
