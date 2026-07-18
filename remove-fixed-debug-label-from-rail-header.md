---
title: Remove debug-only FIXED marker from the rail header
status: backlog
source: task 91 captain-live AC-I1 observation 2026-07-18
sprint: s1-managed-tab-safety
group: narrow-ux
sprint-readiness: defer
started:
completed:
verdict:
score: 0.35
worktree:
issue:
pr:
mod-block:
id: 1zhcvrj8727eez45mdj6ec3j
---

## Problem

The fixed-width rail currently renders `FIXED` in its production header. The
captain accepted the 28-column layout and task 91's nonblocking behavior, but
identified this word as a debug marker rather than intended product copy. It
must not block task 91 or reopen the fixed-width mechanism.

## Required outcome

Replace the production `FIXED` marker with the intended ordinary rail header
copy. Preserve exact 28-column geometry, inert header clicks and former-toggle
inputs, native fullscreen restoration, row click targets, exact-pane session
authority, and all standing-configuration isolation guarantees.

## Acceptance criteria

**AC-1 — production header contains no debug marker.** The rendered header at
the shipped width omits `FIXED` and matches an exact-byte product-copy fixture.

**AC-2 — behavior remains unchanged.** Focused header tests and the narrow
fixed-width smoke prove 28-column geometry, inert input, click-target mapping,
fullscreen restoration, and unchanged standing roots.

## Test plan

Change the exact header-byte expectation red-first, apply the smallest render-
only correction, run focused Rust render/click tests, and exercise the narrow
fixed-width smoke in a fresh disposable tab.

## Out of scope

Width changes, dock/sliver restoration, metadata architecture, session
delivery, new controls, layout mutation, or blocking task 91.
