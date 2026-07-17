---
id: tjj3aqdq4c4at5wrke8cmk0v
title: Keep the managed rail at one fixed width without blocking native fullscreen
status: backlog
source: captain direction after live dirty-tab layout corruption investigation, 2026-07-17
sprint: s1-managed-tab-safety
group: layout-stability
sprint-readiness: ready
started:
completed:
verdict:
score: 0.98
worktree:
issue:
pr:
mod-block:
---

## Problem

The managed rail currently toggles between fixed widths by replacing or cycling the whole tiled layout. In tabs with additional panes, that operation can flatten or stack the user layout and misplace tab-bar/status-bar chrome. The first safe product step is to keep the rail docked at one fixed expanded width, make toggle input incapable of restructuring the tab, and preserve Zellij native fullscreen for ordinary and rail panes.

## Captain constraint

Prefer layout preservation over reclaiming the rail columns. Assume native pane fullscreen remains supported, and verify that assumption as part of the design and live acceptance proof.
