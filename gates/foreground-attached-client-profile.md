---
subspace: v0
gate:
  workflow: agent-rail-dev
  entity: foreground-attached-client-profile
  entity-title: Foreground attached-client disposable Zellij profile
  stage: validation
  round: 1
recommendation:
  verdict: REJECTED
  rationale: "AC-O1's raw-PTY proof flakes: validation observed zero-terminal and absent-canary failures at clean SHA 4320b2f, and a target-free clone timed out before PROFILE_ROOT. AC-I1 remains for CL only after offline stability returns."
artifact:
  kind: draft
  path: ./foreground-attached-client-profile-validation.md
criteria:
  bar: |
    A fresh validator must reproduce AC-O1 through AC-O6, attack the profile
    from a throwaway checkout, and leave the attached-terminal observation to
    CL. A single successful raw-input run does not establish AC-O1.
  acceptance:
    - id: AC-O1
      text: "A test-owned PTY raw write appears in exactly one real terminal's live dump-screen."
      evidence: "REFUTED: repeated real runs saw zero terminals and a nonce absent from 243 completed live dumps."
    - id: AC-O2
      text: "The direct attached client is its own foreground PTY process group before lease publication."
      evidence: "PASS in successful runs: driver observed CLIENT_PID == getpgid(CLIENT_PID) == tcgetpgrp(master) before lease consumption."
    - id: AC-O3
      text: "ProfileLeaseV1 is exact, immutable, private, and bounded."
      evidence: "PASS in successful runs; writable completed lease mutation was rejected and cleanup removed its private state."
    - id: AC-O4
      text: "Normal completion removes disposable state and preserves standing config/layout bytes."
      evidence: "PASS in successful runs, including a manual missing-standing-root normal-cleanup probe."
    - id: AC-O5
      text: "Foreground HUP, TERM, and INT remove client state, session, root, and lease."
      evidence: "PASS in successful runs; forced launcher-loss group cleanup also passed once."
    - id: AC-O6
      text: "No controllable terminal fails before disposable state exists."
      evidence: "PASS: profile-no-tty rejected the invocation before root, session, or lease creation."
    - id: AC-I1
      text: "CL can type a marker in the foreground disposable terminal and exit cleanly."
      evidence: "NOT RUN: exact CL script is in the artifact; do not run while AC-O1 is refuted."
---

Validation at `4320b2f9fadb27d500097b31dc85c76a083918fa` rejects the gate.
The focused packet passes intermittently but cannot establish its most important
end value reliably: the raw PTY nonce sometimes reaches no terminal, and in
another run reaches a terminal that never displays it in live `dump-screen`.

The detailed artifact records the successful cleanup, isolation, identity, and
immutable-lease evidence; the fresh-checkout limitation; the throwaway mutation
audit; and CL's deferred AC-I1 script. Route back to implementation for a
stronger terminal-input readiness condition and repeatable target-free proof.
