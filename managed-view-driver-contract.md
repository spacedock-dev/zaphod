---
id: qbf5syzpwvpggp2xgnd5asvf
title: Managed-view binding and shared driver contract
status: ideation
source: evergreen workspace architecture delivery step 1, captain-authorized 2026-07-11
started: 2026-07-11T04:15:01Z
completed:
verdict:
score: 0.99
worktree:
issue:
pr:
mod-block:
---

## Problem

The managed-view spike proved Zellij mechanisms, but production work has no portable contract for binding one workspace to one multiplexer session and one stable managed view. Define the smallest contract that lets the launcher, Zellij controller, future tmux driver, and shared test suite agree without importing hub, dock, or provider concerns.

## Seed direction

Ideation must define binding identity and persistence, `ensure_managed_view`, list/focus/open behavior, driver-neutral errors and capabilities, stable-ID invalidation, fake-driver fixtures, and the boundary between portable launcher logic and native controller operations. It must propose the corresponding revision to the logical dispatch sequence in `docs/plan-agent-rail.md`.

## Dependency boundary

This task is the foundation. It must not implement the Zellij controller, pane adoption, hub, dock, providers, or tmux. Controller implementation may begin only after this contract's ideation gate is approved.

