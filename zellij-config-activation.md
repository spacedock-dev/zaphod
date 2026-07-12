---
id: fqjswvmd93vek5y12zemf1k2
title: Activation preserves valid Zellij KDL
status: ideation
source: staff review of fp merge 2026-07-12; captain direction
sprint: s1-managed-tab-safety
group: hardening
sprint-readiness: ready
started: 2026-07-12T14:33:06Z
completed:
verdict:
score: 0.98
worktree:
issue:
pr:
mod-block:
---

## Problem

The live managed-tab entry rewrites standing Zellij KDL through a 437-line AWK brace counter. A valid unrelated value such as `WriteChars "{"` causes activation to fail before write. It is fail-closed, but an operator with a normal config can be unable to create a managed tab.

## Required outcome

An operator can run the managed-tab entry against valid supported Zellij configuration, including quoted braces and unrelated bindings; activation either produces a validated config atomically or leaves the standing config and layout byte-for-byte unchanged with a clear error.

## Proposed approach

Replace the hand-rolled AWK structural transformer with a testable Go activation path or a real parser selected by an explicit spike. Preserve the current script as the user entry and preserve its atomic candidate-before-replace behavior. Do not introduce a controller, lease, custom PTY, or Sprint 2 dependency.

## Acceptance criteria

**AC-1 — Valid quoted KDL activates.**
Verified by: a black-box entry-script fixture containing `WriteChars "{"` activates successfully and the resulting config passes `zellij setup --check`.

**AC-2 — Unrelated config survives exactly.**
Verified by: fixture assertions compare unrelated keybind and config blocks before and after activation; only the documented Zaphod bindings and layout reference may differ.

**AC-3 — Invalid input fails without mutation.**
Verified by: malformed fixture input makes activation fail and preserves pre-run config/layout hashes.

## Test plan

First spike a maintained Go/KDL parser or the smallest constrained transformation that round-trips the real fixture. Write red black-box script fixtures for quoted braces and malformed input, then replace the transformer minimally and rerun the entry and isolated tmux smoke.

## Out of scope

Concurrent-activation locking, multi-client behavior, managed-tab route authorization, Sprint 2 bb/session work, gate pooling, 7h, 4d, leases, and custom PTYs.
