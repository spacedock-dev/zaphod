---
id: 1s5n4b0vq2mqcvv8j8gzzfn9
title: Explicit Zellij pane adoption into the managed tab
status: ideation
source: evergreen workspace architecture delivery step 3 and managed-tab option-2 spike, captain-authorized 2026-07-11
started: 2026-07-11T04:21:34Z
completed:
verdict:
score: 0.95
worktree:
issue:
pr:
mod-block:
---

## Problem

Users need an explicit way to bring an existing terminal into the managed tab without restarting its process or retrofitting its foreign tab. Design pane adoption around the proved `break_panes_to_tab_with_id` seam while preserving pane identity, process identity, unrelated panes, and user-controlled permission consent.

## Seed direction

Ideation must define the explicit command/controller request, source and target validation, permission-result state machine, empty-source-tab behavior, visible failure cases, and reusable driver acceptance tests. It must use the existing spike evidence and identify only the remaining production integration drill.

## Dependency boundary

This task follows the stable managed-view identity and controller message path. It must not automate permission responses, auto-adopt panes, revive foreign-tab retrofit, or add hub, dock, provider, or tmux behavior.

