---
subspace: v0
gate:
  workflow: agent-rail-dev
  entity: fixed-width-managed-rail
  entity-title: Keep the managed rail at one fixed width without blocking native fullscreen
  stage: validation
  round: 1
recommendation:
  verdict: REJECTED
  rationale: "O1-O3 pass, but O4 is refuted because the isolated smoke writes its candidate-bearing server log under the host default Zellij socket/log root; an extra responsiveness run also exposed intermittent cursor-state proof drift."
artifact:
  kind: draft
  path: ./fixed-width-managed-rail-validation.md
criteria:
  bar: |
    A fresh validator must verify the stored exact-range panel, independently
    reproduce O1-O4, attack mouse/keybind/fullscreen/identity/chrome/root
    boundaries from a throwaway checkout, and leave I1-I2 to the captain.
  acceptance:
    - id: O1
      text: "A populated managed tab has one stable fixed rail and intact chrome."
      evidence: "PASS: both permission modes reached the exact six-pane 160x48 inventory with one canonical 28x46 rail and two exact chrome rows."
    - id: O2
      text: "Former toggle inputs cannot mutate pane layout."
      evidence: "PASS: literal Alt /, the delivered FIXED click, stale CLI pipe, and granted active-tiled Keybind-source toggle were inert."
    - id: O3
      text: "Native fullscreen round-trips both pane kinds without layout loss."
      evidence: "PASS in the required packet; one of five extra responsiveness runs failed only on terminal cursor coordinates, exposing proof instability."
    - id: O4
      text: "Entry, row interaction, permission behavior, recipient isolation, and standing-root isolation do not regress."
      evidence: "REFUTED: the host default $TMPDIR/zellij-501/zellij-log/zellij.log records the detached candidate's smoke server lifecycle."
    - id: I1
      text: "Extra panes remain arranged after former toggle inputs in WORK."
      evidence: "NOT RUN: exact captain script is in the artifact; offline rejection blocks the live demo."
    - id: I2
      text: "The captain's native fullscreen journey works for ordinary and rail panes."
      evidence: "NOT RUN: exact captain script is in the artifact; offline rejection blocks the live demo."
---

Validation at `2f209781c10184a768292555aea0effc8aa77145` rejects the
gate. Product behavior passed O1-O3 in the required runtime packet, but O4's
standing-root promise is false for the current harness: Zellij control calls
redirect socket/data/config while leaving `HOME` and `TMPDIR` standing, and the
host default Zellij log contains the detached candidate's server lifecycle.

The detailed artifact contains the exact review evidence, per-AC commands,
named adversarial probes, intermittent cursor-snapshot finding, and the
captain-driven I1-I2 script to run only after offline isolation is repaired.
