---
id: fp8pn84km859qges2s2ffp5h
title: Zellij managed-tab controller and guarded keybindings
status: ideation
source: evergreen workspace architecture delivery step 2 and managed-tab option-2 spike, captain-authorized 2026-07-11
started: 2026-07-11T04:15:02Z
completed:
verdict:
score: 0.98
worktree:
issue:
pr:
mod-block:
---

## Problem

Zaphod has no shipped entry point that creates or focuses one managed Zellij tab, and the current `Alt /` prototype still mutates ordinary tabs. Design the thin native controller and portable launcher seam that make `Alt Shift z` idempotent and make `Alt /` a no-op outside the recorded managed tab.

## Seed direction

Ideation must reuse the completed live spike evidence, define controller messages and permission flow, specify artifact preflight and visible failures, extend the disposable Zellij profile, and split offline from interactive acceptance. The CLI `Run` path remains a proving harness, not the production keybinding mechanism.

## Dependency boundary

Ideation may proceed in parallel with the driver contract to expose interface pressure. Implementation must consume the approved managed-view contract and must not add pane adoption, hub, dock, provider, or tmux behavior.

