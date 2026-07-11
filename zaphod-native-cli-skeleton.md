---
id: bc7r9aj9s4a799dxt0qvebfq
title: Native zaphod CLI skeleton and artifact ownership
status: backlog
source: managed-view roadmap Sprint 1 foundation, senior staff review 2026-07-11
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

The managed-view designs call an installed `zaphod` helper, but no task owns its language, package, artifact path, build/install behavior, version handshake, structured command envelope, or disposable-profile wiring.

## Sprint role

Choose and ship the smallest testable native CLI artifact without managed-tab behavior. It must expose version/capability identity and typed command/result plumbing usable by the binding core and Zellij feasibility harness. It must build from a worktree without mutating standing installation or global Zellij state.

