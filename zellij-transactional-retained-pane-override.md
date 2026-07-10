---
title: Zellij transactional retained-pane override for safe foreign-tab docking
status: ideation
score: 0.95
source: j5 cycle-5 host-API blocker — captain selected Choice A, 2026-07-10
started: 2026-07-10T15:03:06Z
completed:
worktree:
issue:
pr:
verdict:
mod-block:
id: 4dt6vkakec3fmmpm5s0kgtch
---

## Problem

Zellij 0.44.3 retained layout override drains unmatched panes and reinserts them one by one without preflight, rollback, or a result channel. An unsatisfiable insertion can drop an owned terminal, while the public plugin surfaces omit the original typed launch identity needed to avoid duplicate spawns. Zaphod cannot safely dock a rail into a foreign tab until the host can bind every retained pane to valid non-spawning positions atomically or leave the original tab untouched.

## Proposed approach

Design and exercise the smallest server-owned transactional retained-pane operation against the exact j5 red cases. Prefer an upstreamable Zellij patch with a result-bearing plugin/CLI seam; use a local fork only as a disposable proof unless a later gate explicitly approves shipping it. The operation must preflight complete pane placement, commit all retained panes plus the rail, or make no change.

## Acceptance criteria

To be completed in ideation from j5 AC-1 through AC-5, including N=1/N=2/N=3 ID preservation, atomic failure, pre-swap base safety, launch-identity variants, and a result-bearing host contract.

## Test plan

Start with the existing clean N=2 unsatisfiable-layout reproduction: the unpatched host loses a terminal; the candidate operation must return failure with identical pane IDs, processes, geometry, and swap state. Then cover successful multi-pane insertion and the full j5 identity matrix before any Zaphod integration.

## Out of scope

- Heuristic post-hoc terminal cleanup.
- Reading current cwd or command as a substitute for original typed launch identity.
- Shipping a Zellij fork without a later explicit gate.
- eh's floating-instance leak and chrome-race work.
