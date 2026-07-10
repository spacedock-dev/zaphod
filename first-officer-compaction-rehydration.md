---
id: njr36mfyhbafy8zx9ydks8ep
title: Deterministic first-officer rehydration after compaction
status: backlog
source: captain finding — first-officer critical-path wait contract degraded after context compaction, 2026-07-11
started:
completed:
verdict:
score: 0.95
worktree:
issue:
pr:
mod-block:
---

## Problem

Conversation compaction can preserve a narrative summary while weakening an operational first-officer invariant. In the observed failure, the compacted context mentioned Codex wait semantics, but the first officer dispatched a critical-path feedback repair and returned before consuming its completion signal, verifying durable output, and advancing to re-review. A compact summary is not a reliable substitute for re-reading the authoritative workflow contracts.

## Proposed approach

Design a deterministic rehydration protocol for Spacedock first officers. It should detect the post-compaction condition available on each supported runtime, re-read the minimum authoritative skill/runtime/state sources before action, restore unresolved-worker and gate obligations, and make critical-path continuation mechanically auditable. Compare instruction-only guidance, a durable checkpoint/sentinel, lifecycle hooks, and app-server event injection; prefer the smallest cross-runtime contract with an enforceable v2 path where available.

## Acceptance criteria

To be designed during ideation. At minimum, the design must externally prove that an unresolved critical-path worker cannot be treated as complete or abandoned after compaction, and must define a bounded fallback where a runtime exposes no compaction event.

## Test plan

Ideation must specify a replayable pre-compaction/post-compaction scenario covering dispatch, compaction, completion delivery, durable-state verification, and next-gate continuation. It must include adversarial cases for summary omission, queued completion, operator interruption, and runtimes without event injection.

## Out of scope

Implementing the protocol in this stage; changing Zaphod product behavior; weakening existing first-officer wait or feedback-routing contracts.
