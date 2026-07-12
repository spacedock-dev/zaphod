---
id: v3d4m3zfryre1eypjnaetd37
title: Alt slash only changes a Zaphod-created tab
status: ideation
source: staff review of fp merge 2026-07-12; captain direction
sprint: s1-managed-tab-safety
group: walking-skeleton
sprint-readiness: ready
started: 2026-07-12T14:50:58Z
completed:
verdict:
score: 1.0
worktree:
issue:
pr:
mod-block:
---

## Problem

The shipped managed-tab entry permits a visible, tiled Zaphod-sidebar-shaped plugin to install and receive the runtime `Alt /` route without proving that it is the rail created by the managed-tab entry. A lookalike or unmanaged resident can therefore affect its own tab. This is a safety failure in the existing operator outcome, not Sprint 2 work.

## Required outcome

An operator can use `Alt /` in a fresh tab created by `scripts/zellij-new-tab.sh`; it changes only that tab’s known rail state. In every unmanaged, foreign, floating, absent, stale, or replaced-rail context, the same key is inert: no layout override, pane creation, focus change, or plugin reload.

## Proposed approach

Design the smallest explicit managed-tab authorization proof and require it both when the resident offers its runtime route and when it receives a toggle pipe. Visible tiled state, a title, a CWD, and a URL substring alone are insufficient. Extend the existing tmux-hosted smoke with an unmanaged tiled/sidebar-bearing adversary and preserve the direct `Alt Shift z` entry. Do not introduce a lease, custom PTY, controller, or binding-core.

## Acceptance criteria

**AC-1 — A Zaphod-created tab remains usable.**
Verified by: an isolated real-Zellij tmux smoke sends literal `Alt Shift z` then `Alt /` and observes the expected visible rail and native layout transition.

**AC-2 — Authorization distinguishes managed from merely similar.**
Verified by: focused tests and a real-Zellij adversarial smoke show an active tiled sidebar-bearing unmanaged or lookalike resident cannot install or consume the toggle route; a URL substring, title, CWD, or geometry alone cannot authorize it.

**AC-3 — Foreign safety holds after lifecycle changes.**
Verified by: the smoke exercises a foreign, floating, stale, or same-URL replacement context and proves literal `Alt /` leaves layout, panes, focus, and plugin loading unchanged.

## Test plan

First spike the smallest real native state that can distinguish the entry-created rail from an unmanaged tiled rail. Then write the red unit/integration assertions, make the minimal route and receipt change, and run the existing isolated tmux smoke plus the new adversarial case. Include a captain-live `WORK` drill for the reported client behavior at validation.

## Out of scope

Sprint 2 bb session ingestion and `zaphod subscribe`; the AWK config-rewriter/concurrency review findings; multi-client feature expansion; 7h, 4d, leases, custom PTYs, controllers, and binding cores. This follow-up does not block bb.
