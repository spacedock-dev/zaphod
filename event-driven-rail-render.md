---
id: stevm8fga4tnn1njgzw4ht4g
title: Rail renders only on state-changing events
status: backlog
source: zj-radar comparative research finding (2026-07-16)
sprint:
group:
sprint-readiness:
started:
completed:
verdict:
score:
worktree:
issue:
pr:
mod-block:
---

## Problem

The rail re-renders on a fixed ~1.2 s synchronous cadence regardless of whether anything changed. That cadence is implicated in KJ's AC-O5 congestion failures — heartbeat delivery aligns with the synchronous refresh so a literal `Alt n` missed the one-second deadline (managed-tab-safety-session-integration, cycles 9/10), and cadence tuning provably cannot fix it. External evidence the cadence is unnecessary: zj-radar (/Users/clkao/git/spacedock-research/spaceterm/zj-radar) renders a comparable per-tab rail with no periodic refresh — every render responds to a pushed input (status pipe, PaneUpdate/TabUpdate, CommandChanged); its only recurring timer drives animation/presence at 1 Hz. Its poll-driven predecessor melted a ~30-pane session (docs/smart-tabs-postmortem.md:16-44).

## Proposed approach

Render only in response to state-changing inputs: pipe payloads, PaneUpdate/TabUpdate, snapshot/lease transitions. Time-based state changes (lease expiry, row TTLs) arm a timeout when the state is created rather than being discovered by a polling tick. Depends on task 91 (nonblocking-pane-metadata-architecture) as the event-only pane-state authority; this task removes the render cadence on top of it.

## Acceptance criteria

**AC-1 — No fixed-interval refresh remains.**
Every render call site is traceable to a triggering event or armed timeout. Verified by: grep for the refresh timer in the plugin source plus a render call-site inventory in the stage report.

**AC-2 — Lease fail-close timing preserved.**
Watcher loss still expires rows within the 2.5 s lease bound without a polling tick. Verified by: existing KJ lease tests rerun green, or a test driving the timeout path.

**AC-3 — Congestion drill passes.**
With watcher and rail live, the KJ AC-O5 drill (literal `Alt n` within one second) passes. Verified by: rerun of the congestion panel from task 91 / KJ.

## Test plan

Host-side tests over the event→render mapping (cheap); one live congestion drill shared with KJ's AC-O5 rerun.

## Out of scope

Hook-based status ingestion, identity/trust model changes, and task 91's snapshot mechanism itself (a dependency, not scope).
