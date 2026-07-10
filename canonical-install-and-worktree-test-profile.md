---
title: Canonical install and isolated worktree test profile
status: ideation
source: finding — runtime audit found layout/keybind plugin identity split, 2026-07-10
started: 2026-07-10T12:38:46Z
completed:
verdict:
score: 0.95
worktree:
issue:
pr:
mod-block:
id: v9hwpzcc1cteqfdgta461f5f
---

## Problem

The global zaphod layout can be installed from an unmerged Git worktree while
the user's keybinds still target the canonical checkout. Zellij keys plugin
instances by URL plus configuration, so this produces distinct resident and
message-launched plugin identities and invalidates live testing. The workspace
needs one verifiable canonical install and a separate, disposable way to build
and exercise an unmerged worktree without mutating `~/.config/zellij`.

This task owns install provenance, identity verification, an isolated
worktree-test profile, regression coverage for those boundaries, and the
operator documentation/j5 drill that uses the profile. It does not change the
runtime layout transform (j5) or investigate chrome corruption and leaked
instances (eh).
