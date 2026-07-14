---
subspace: v0
gate:
  workflow: agent-rail-dev
  entity: managed-tab-safety-session-integration
  entity-title: Integrate managed-tab safety with tab-bound session delivery
  stage: validation
  round: 1
recommendation:
  verdict: PENDING-DEMO
  rationale: "AC-O1 through AC-O6 independently pass at frozen head dec685e8; AC-I1 remains the captain's fresh-session two-tab manual watcher demo."
artifact:
  kind: draft
  path: ./managed-tab-safety-session-integration-validation.md
criteria:
  bar: |
    The exact-head code_completion packet, all six offline criteria, and a
    throwaway adversarial audit must pass before the captain spends time on
    AC-I1. Approval then requires the captain's direct two-tab observation;
    offline harness output cannot substitute for that live result.
  acceptance:
    - id: AC-O1
      text: Manual startup resolves one exact live authority tuple per watched same-CWD tab and none for a foreign shell.
      evidence: "PASS: native two-rail and lifecycle smokes independently reproduced exact tab/terminal/rail separation."
    - id: AC-O2
      text: The private socket admits only a matching top-level SessionStart and keeps one in-memory registration.
      evidence: "PASS: full-record, socket, over-limit, wrong-authority, child, replacement, and no-registry tests passed."
    - id: AC-O3
      text: Exact projection yields 1,1,0 and row clicks focus the exact watched pane.
      evidence: "PASS: native two-rail smoke proved exact requests, child/global-list negatives, screen cardinality, and literal focus."
    - id: AC-O4
      text: Loss of live authority clears rows and focus within the lease without recovery state.
      evidence: "PASS: watcher/terminal/original-rail loss, bystander, restart-empty, lease, and cleanup matrices passed."
    - id: AC-O5
      text: Managed ownership remains exact and idle watcher renewal does not delay normal Zellij actions.
      evidence: "PASS: responsive, congestion, managed-toggle, no-idle-query, and injected-timeout cleanup proofs passed."
    - id: AC-O6
      text: Failure and cleanup leave no test-owned authority artifact, helper, session, or standing-file change.
      evidence: "PASS: four stress cases and native cleanup assertions passed; candidate worktree remained clean."
    - id: AC-I1
      text: The captain can use two manual same-CWD watchers with exact rows/focus, child/history exclusion, lease removal, and restart-empty behavior.
      evidence: "PENDING: captain-live script is in the artifact; only the captain's reported observation settles it."
---

Offline validation is green at `dec685e8cf6efbc84ef2b26f60a81e4eda9aacf5`.
The stored `code_completion` panel covers the exact merge-base range, contains
correctness, journey, and proof once each with no execution failure, and has a
passing synthesis verdict. Its removed ephemeral endpoint was an evidence
retention defect; the immutable raw `roborev show` tool result supplies the
same packet without a rerun or new lifecycle layer.

The artifact records the independent command packet, six named adversarial
attacks, and the exact fresh-session AC-I1 script. Keep this gate pending until
the captain reports the live row, focus, lease, restart, and cleanup outcomes.
