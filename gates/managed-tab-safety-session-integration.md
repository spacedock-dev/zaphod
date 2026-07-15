---
subspace: v0
gate:
  workflow: agent-rail-dev
  entity: managed-tab-safety-session-integration
  entity-title: Integrate managed-tab safety with tab-bound session delivery
  stage: validation
  round: 2
recommendation:
  verdict: REJECTED
  rationale: "AC-O5's native congestion proof failed twice at literal Alt n, and Roborev parent 1653 covers only bd18165 rather than the required 9343129..bd18165 range. AC-I1 is held."
artifact:
  kind: draft
  path: ./managed-tab-safety-session-integration-validation.md
criteria:
  bar: |
    The exact-range code_completion packet and all six offline criteria must
    pass before the captain runs AC-I1. The real congestion packet must meet
    its native-state deadline during an in-flight refresh; pure predicates do
    not replace that caller-impact proof.
  acceptance:
    - id: AC-O1
      text: Manual startup resolves one exact live authority tuple per watched same-CWD tab and none for a foreign shell.
      evidence: "PASS: native two-rail and focused startup tests reproduced exact tab, terminal, and original-rail separation."
    - id: AC-O2
      text: The private socket admits only a matching top-level SessionStart and keeps one in-memory registration.
      evidence: "PASS: full-record, socket, over-limit, wrong-authority, child, replacement, and no-registry tests passed."
    - id: AC-O3
      text: Exact projection yields 1,1,0 and row clicks focus the exact watched pane.
      evidence: "PASS: native two-rail smoke proved exact requests, child/global-list negatives, screen cardinality, and literal focus."
    - id: AC-O4
      text: Loss of live authority clears rows and focus within the lease without recovery state.
      evidence: "PASS: manifest, original-rail, bystander, restart-empty, lease, and cleanup matrices passed."
    - id: AC-O5
      text: Post-ready native inventory stays zero and managed pane/tab actions remain responsive.
      evidence: "REFUTED: inventory counts passed, but the native congestion packet failed twice at literal Alt n's one-second complete-tab deadline."
    - id: AC-O6
      text: Failure and cleanup leave no test-owned authority artifact, helper, session, or standing-file change.
      evidence: "PASS: four injected stress cases and native cleanup assertions passed."
    - id: AC-I1
      text: The captain can use two manual same-CWD watchers with exact rows/focus, child/history exclusion, lease removal, and restart-empty behavior.
      evidence: "NOT RUN: blocked by AC-O5 and incomplete review-range coverage; the held script is in the artifact."
---

Validation rejects frozen head
`bd181658e06b917964e5697580cd0d198266ac99`. The detached exact-head packet
passed Go, Rust 147/147, build, hook, entry, two-rail `1/1/0`, outside/inside
lifecycle, layout, and four cleanup-stress cases. The real congestion packet
failed twice at the same literal `Alt n` boundary.

Stored parent 1653 is structurally complete and PASS, but its patch ID equals
the final commit alone. The required merge-base range has a different patch
ID. Route back to implementation for the congestion repair and a replacement
full-range `code_completion` packet. The captain-live script is prepared but
must remain held.
