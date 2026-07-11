---
id: m132a4zczf14b0b8dmwtfnmh
title: Upstream Zellij transactional retained-pane operation
status: backlog
source: parked upstream ask from 4d validation and managed-tab architecture decision, 2026-07-11
started:
completed:
verdict:
score: 0.45
worktree:
issue:
pr:
mod-block:
---

## Problem

Zellij 0.44.3 runtime layout override can destructively remove retained terminal panes before discovering that the replacement layout cannot seat them. In Zaphod's clean N=2 reproduction, one retained override changed two terminals into one terminal plus the rail after `UnsatisfiableConstraint` / `Not enough room for another pane`; no concurrent plugin leak or chrome corruption was present. Zellij also does not expose enough original typed launch identity for Zaphod to reconstruct an exact concrete layout safely.

The local 4d proof established that pure planner, commit, and result helpers are insufficient. A real operation crosses Zellij's action/plugin API, IPC/protobuf, screen routing, PTY/plugin staging, tab mutation, and result-delivery boundaries. This task records that upstream requirement; it does not authorize a local fork, block the managed-tab architecture, or request implementation now.

## Upstream requirement

Provide one result-bearing transactional retained-pane layout operation with these properties:

1. The caller targets a stable tab ID, not the transient CLI client's active-tab association.
2. Zellij snapshots every retained pane's stable ID and mutation-relevant fingerprint before changing the tab.
3. The entire candidate layout is planned against the current viewport before any retained pane is removed, inserted, spawned, unloaded, or dropped.
4. Plugin panes may be staged without interpreting empty or `children` layout slots as terminal-spawn requests.
5. Immediately before commit, Zellij rechecks the target tab ID and retained-pane fingerprint; stale plans reject without mutation.
6. A successful commit applies the exact plan once. An impossible or stale plan leaves pane IDs, geometry, processes, plugins, and tab chrome unchanged.
7. The real plugin and CLI caller paths receive an explicit `Applied` or `Rejected { reason }` result. CLI rejection exits non-zero and writes an actionable diagnostic.
8. Tests exercise feasible and impossible N=1/N=2/N=3 layouts through the real server operation and externally observe pane IDs, geometry, process survival, zero unintended terminal spawn/drop, and caller-visible results.

## Relevant upstream references

- Runtime override implementation: zellij-org/zellij PR #4566 — https://github.com/zellij-org/zellij/pull/4566
- Layout-manager and synchronous plugin API: PR #4601 — https://github.com/zellij-org/zellij/pull/4601
- Transient CLI client resolves no active tab and exits zero: issue #5250 — https://github.com/zellij-org/zellij/issues/5250
- General runtime-layout history: issue #742 — https://github.com/zellij-org/zellij/issues/742
- New-tab/default-template integration, relevant to the managed-tab alternative: issue #4646 — https://github.com/zellij-org/zellij/issues/4646
- Local evidence and rejected helper-only proof: task `zellij-transactional-retained-pane-override` (4d), including its validation artifact and cycle-3 call-graph blocker.

No searched upstream issue or pull request currently covers the full transactional retained-pane contract above. Before filing externally, recheck upstream main and issue/PR search, reduce the local reproducer to a standalone Zellij case, and confirm maintainers' preferred API/result vocabulary.

## Acceptance criteria

**AC-1 — Standalone upstream report.** The proposed issue or RFC reproduces the N=2 loss without Zaphod, names the current destructive call path, and distinguishes the requirement from #5250's transient-client targeting defect.
Verified by: run the reproduction against the then-current upstream release and main, attaching before/after pane/process evidence and logs.

**AC-2 — Transactional contract is externally testable.** The ask specifies stable targeting, preflight, stale-plan rejection, atomic commit, zero spawn/drop on rejection, and result propagation as observable behavior rather than helper API shape.
Verified by: an upstream-oriented test matrix covering N=1/N=2/N=3 feasible and impossible layouts through the real CLI/plugin-to-server path.

**AC-3 — No accidental product dependency.** Filing or discussing the upstream ask does not make Zaphod depend on a fork or restore foreign-tab retrofit as a release prerequisite.
Verified by: Zaphod's active architecture and delivery plan continue to use the managed tab/window path unless a later captain-approved decision explicitly changes them.

## Test plan

When unparked, first refresh the upstream search and reproduce on current main. Then extract 4d's smallest clean N=2 case into an upstream-native fixture, record the existing failure, and draft the issue/RFC around observable semantics. Do not build a new patch series until maintainers agree on the operation boundary.

## Out of scope

Implementing or maintaining a Zellij fork; changing Zaphod's managed-tab design; shipping the 4d proof kit; fixing unrelated layout-manager UI, default-template, or transient-client defects; filing the external issue without a fresh captain gate.
