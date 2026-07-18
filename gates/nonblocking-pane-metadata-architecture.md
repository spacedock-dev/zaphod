---
subspace: v0
gate:
  workflow: agent-rail-dev
  entity: nonblocking-pane-metadata-architecture
  entity-title: Remove synchronous pane metadata calls from the plugin hot path
  stage: validation
  round: 1
recommendation:
  verdict: PENDING_CAPTAIN
  rationale: "AC-O1 through AC-O6 independently pass at frozen head 45719f4 with exact-range code_completion parent 188. AC-I1 remains reserved for the captain's real Subspace/Codex fixed-width journey."
artifact:
  kind: draft
  path: ./nonblocking-pane-metadata-architecture-validation.md
criteria:
  bar: |
    The captain may approve only after the real fresh-session AC-I1 preserves
    the 28-column rail, exact delivered-row focus, sub-one-second pane/tab
    actions during held metadata enrichment, restart-empty recovery, a
    six-second quiet window, and complete cleanup.
  acceptance:
    - id: AC-O1
      text: Slow enrichment cannot delay or burst pane and tab actions.
      evidence: "PASS: fixed 28-column exact-row focus, literal action deadlines, held barrier, negative controls, and quiet window reproduced."
    - id: AC-O2
      text: WASM performs no synchronous pane metadata lookup.
      evidence: "PASS: Rust 80/80 includes zero-forbidden-call, permission, source, and every handler-path matrix."
    - id: AC-O3
      text: Scheduling, cancellation, and cache size stay bounded.
      evidence: "PASS: fresh uncached Go coordinator suite and vet passed."
    - id: AC-O4
      text: Exact per-tab pane authority fails closed across lifecycle and restart.
      evidence: "PASS: native two-rail smoke proved exact 1/1/0, restart-empty, fresh recovery, exact focus, and zero idle polls."
    - id: AC-O5
      text: Stale and malformed data degrade without lying.
      evidence: "PASS: Rust atomic validator/projection and Go outage/recovery matrices passed malformed, stale, sequencing, limit, and recovery cases."
    - id: AC-O6
      text: Failure and cleanup remain isolated.
      evidence: "PASS: deadline, forced failure, native hang, vanished startup, and cleanup-probe controls retained evidence and removed owned state."
    - id: AC-I1
      text: The captain can work normally beside a real slow Subspace pane in a fresh managed tab.
      evidence: "PENDING: exact captain-live script is in the validation artifact; no interactive observation is claimed."
---

Frozen head `45719f4` passes every independently reproducible offline
criterion, the local negative correctness audit, and the exact-range review
integrity check. Present the linked artifact and its exact AC-I1 script to the
captain. The decision log must record the real interactive result; offline
harness evidence is not a substitute for the captain's TUI observation.
