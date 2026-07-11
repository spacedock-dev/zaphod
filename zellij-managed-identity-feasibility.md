---
id: 6vbb1n6rpv6qy0hs6z3avctx
title: Zellij managed identity and marker feasibility spike
status: backlog
source: managed-view roadmap Sprint 1 native feasibility lane, senior staff review 2026-07-11
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

Production controller work depends on unproved Zellij identity mechanisms: an exact multi-client key invocation witness, a durable marker owner and query path, a stable session-incarnation representation, and recovery across create-before-persist crashes and native ID reuse.

## Sprint role

Run a disposable, throwaway feasibility spike after the foreground-profile gate. Prove or reject invocation witness, persistent marker ownership/query, detach/reattach and replacement identity, stable-ID reuse, and crash-window recovery. Do not grow the spike into production controller code. Its gate either freezes the Zellij representation for integration with the binding core or returns the architecture for revision.

