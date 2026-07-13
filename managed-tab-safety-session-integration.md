---
title: Integrate managed-tab safety with tab-bound session delivery
status: ideation
group: walking-skeleton
sprint: s1-managed-tab-safety
sprint-readiness: ready
score: 1.0
source: captain direction 2026-07-13; V3/BB merge conflict
id: kjhq0t2h6drse6b32cqybggv
started: 2026-07-13T06:54:16Z
---

## Problem

The approved managed-only `Alt /` hardening branch and the merged tab-bound
session subscriber branch diverged from the same baseline. Their code merges,
but their README edits conflict, leaving the V3 safety proof absent from
`main` and the standing `Alt Shift z` configuration pointing at a worktree
candidate.

## Required outcome

An operator can use a main-built fresh managed tab that preserves BB's
tab-bound session subscriber and V3's managed-only toggle proof. Documentation
describes both behaviors coherently. This repair does not mutate standing
Zellij config or layouts.

## Acceptance criteria

**AC-1 — Main contains both delivered behaviors.**
Verified by: the integration branch contains BB's private tab-bound subscriber
startup and V3's explicit `zaphod_managed_tab "v1"` plus exact-WASM toggle
authorization.

**AC-2 — The normal safety packet stays green.**
Verified by: Rust tests, entry-script tests, and the tmux-hosted Zellij smoke
pass from the integrated head; the smoke proves the managed toggle and
lookalike inertness.

**AC-3 — The user-facing contract is coherent.**
Verified by: README preserves BB's subscriber lifecycle and states V3's
managed-proof boundary without claiming a global binding selects a worktree
artifact.

## Out of scope

Changing persistent Zellij configuration, replacing the AWK transformer,
reworking the CLI entry path, 7h, 4d, gate pooling, or new session/gate
features. Those stay with their existing tasks.
