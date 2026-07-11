---
title: Pending gates appear where the work came from
status: backlog
source: captain direction 2026-07-11; Sprint 2 outcome shaping
sprint: s2-dependable-per-tab-attention-loop
group: walking-skeleton
sprint-readiness: ready
blocked-on: v1-gate-origin-contract-and-7h-validation-before-implementation
blocked-reason: Ideation is captain-approved now. Implementation needs the gate skill origin contract and the passed disposable-profile gate for its multi-tab drill.
score: 0.97
started:
completed:
verdict:
worktree:
issue:
pr:
mod-block:
id: s92rm4v2pz0m23memjg9xha2
---

## Problem

Pending reviews need truthful attention without forcing the operator to hunt for their originating work or silently losing gates whose provenance is unavailable.

## Required outcome

A gate with verified v1 origin appears only in its originating tab. A gate with no usable origin remains globally visible. The rail reconciles the provider-owned set of open gates so resolution updates or removes the row.

## Ideation boundary

Design the configured source and open-gate reconciliation, carry optional gate-skill-provided origin, match only against an exact proven Zellij identity, and fall back globally for missing or malformed origin. First spike the actual v1 origin carrier. Do not infer origin from path, cwd, title, or a guessed tab identifier; do not render an inline verdict or write a decision log. Use pz only as evidence.
