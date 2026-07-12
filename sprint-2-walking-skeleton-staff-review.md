---
title: Sprint 2 walking-skeleton staff coherence review
status: ideation
source: captain direction 2026-07-12
completed:
verdict:
score: 1.0
worktree:
issue:
pr:
mod-block:
sprint: s2-dependable-per-tab-attention-loop
group: release-gate
sprint-readiness: ready
blocked-on:
id: 9j214d92fg2vxa5wmapxxxbj
started: 2026-07-12T01:58:05Z
---

## Problem

Before Sprint 2 moves from coherent task designs to implementation, an independent staff review must establish whether its session-ingestion, gate-projection, and review-surface tasks compose into one end-user walking skeleton. The review treats the expected fully validated result of Sprint 1 task `7h` as an explicit conditional contract, while preserving the fact that `7h` is currently rejected at validation.

## Required outcome

Produce a cited staff assessment of the Sprint 2 release-gate record plus `bb`, `s9`, and `qt`: their end-user sequence, ownership boundaries, dependency order, and a clear assumption ledger for what a fully validated `7h` does and does not grant. Recommend the smallest coherent Sprint 2 outcome and any deferrals needed to protect it.

## Review boundaries

Do not alter code, roadmap, or any reviewed task. Do not alter `7h` or `4d`. Do not treat `ProfileLeaseV1` as canonical tab identity, session-incarnation proof, a second-client grant, or gate-skill authority unless the reviewed records independently establish that claim.

## Acceptance criteria

**AC-O1 — The review maps each Sprint 2 task to one operator-visible step and identifies every cross-task prerequisite.**

Verified by: a cited task-to-outcome/dependency matrix whose sources are the current task records and roadmap, not assumptions in the review itself.

**AC-O2 — The review separates the counterfactual fully-validated `7h` contract from `7h`'s current rejected validation state.**

Verified by: citations to `7h`'s acceptance criteria, implementation report, and validation report, including both its claimed ProfileLeaseV1 boundaries and its unresolved AC-O1 readiness failure.

**AC-O3 — The review gives one actionable coherence recommendation, including what may progress and what remains deferred.**

Verified by: a proposed demo sequence that names its independent proof and explicitly prohibits unsafe identity, origin, and review-routing inferences.

## Out of scope

Implementing any Sprint 2 task, approving a gate, changing Sprint 1, or replacing the gate skill's external v1 contract.
