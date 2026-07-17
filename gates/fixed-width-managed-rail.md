---
subspace: v0
gate:
  workflow: agent-rail-dev
  entity: fixed-width-managed-rail
  entity-title: Keep the managed rail at one fixed width without blocking native fullscreen
  stage: validation
  round: 2
recommendation:
  verdict: REJECTED
  rationale: "The main smoke repair passes normal, upgrade, and five responsiveness runs with complete disposable roots, but O4 remains refuted because the unchanged recipient smoke writes its exact detached candidate path to the host default Zellij log."
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
      evidence: "PASS: normal, upgrade, and five responsiveness runs reached the exact six-pane 160x48 inventory with one canonical 28x46 rail and two exact chrome rows."
    - id: O2
      text: "Former toggle inputs cannot mutate pane layout."
      evidence: "PASS: literal Alt /, the delivered FIXED click, stale CLI pipe, and granted active-tiled Keybind-source toggle were inert."
    - id: O3
      text: "Native fullscreen round-trips both pane kinds without layout loss."
      evidence: "PASS: seven main runs, including five responsiveness attacks, preserved every stable identity/geometry/chrome/command/fullscreen field while normalizing only focus and cursor."
    - id: O4
      text: "Entry, row interaction, permission behavior, recipient isolation, and standing-root isolation do not regress."
      evidence: "REFUTED: the repaired main harness is isolated, but the passing two-rail recipient smoke changed its exact candidate-path count in the host default Zellij log from 0 to 1."
    - id: I1
      text: "Extra panes remain arranged after former toggle inputs in WORK."
      evidence: "NOT RUN: exact captain script is in the artifact; offline rejection blocks the live demo."
    - id: I2
      text: "The captain's native fullscreen journey works for ordinary and rail panes."
      evidence: "NOT RUN: exact captain script is in the artifact; offline rejection blocks the live demo."
---

Validation cycle 2 at `b9568fc8645646c15bb31878827cb86e4f1fff28`
rejects the gate. The repaired main harness passes O1-O3 and complete root
isolation across normal, upgrade, and five responsiveness runs. O4's standing-
root promise remains false for the unchanged recipient harness, whose passing
server appended the detached candidate identity to the host default Zellij log.

The detailed artifact contains the exact review evidence, per-AC commands,
named adversarial probes, the repaired cursor-normalizer attacks, and the
captain-driven I1-I2 script to run only after recipient isolation is repaired.
