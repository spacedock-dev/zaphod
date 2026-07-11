---
id: 68mb9yq4v90fnb8gwhe1kjwy
title: Senior staff review of the managed-view implementation sprint
status: backlog
source: captain-requested full-sprint review of managed-view ideation and dispatch sequence, 2026-07-11
started:
completed:
verdict:
score: 1.0
worktree:
issue:
pr:
mod-block:
---

## Problem

Three related ideation packets now define the managed-view contract, Zellij controller, and explicit pane adoption, while a live drill exposed a foreground-client defect in the canonical disposable profile. Review the work as one sprint before implementation begins so local task quality does not hide a broken critical path, misplaced gate, unsafe parallelism, or unowned integration risk.

## Review scope

Review `managed-view-driver-contract.md`, `zellij-managed-tab-controller.md`, `zellij-pane-adoption.md`, `docs/zaphod-workspace-architecture.md`, `docs/plan-agent-rail.md`, and `scripts/zellij-worktree-test-profile.sh`. Treat the proposed order as: repair interactive test infrastructure; freeze the portable contract; run contract implementation and the controller invocation-witness spike in parallel; integrate and validate the controller; run the adoption transport invalidation drill; implement adoption; then proceed to hub, dock, tmux, and providers.

## Expected outcome

Return an independent senior-staff recommendation: approve the sprint, approve with concrete reframing, or reject. Name the minimum task splits or dependency changes, the exact critical path and safe concurrency, required gates and evidence, sprint exit criteria, and any work that must be parked. Do not edit product code or product documentation.

