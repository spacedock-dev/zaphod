---
subspace: v0
gate:
  workflow: agent-rail-dev
  entity: fixed-width-managed-rail
  entity-title: Keep the managed rail at one fixed width without blocking native fullscreen
  stage: validation
  round: 3
recommendation:
  verdict: PASSED
  rationale: "O1-O4 pass independently at c269b31: normal and upgrade fixed-width/fullscreen journeys are green, two recipient runs preserve exact stable-tab/token delivery, and candidate-attributable host records remain empty. I1-I2 await the captain."
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
      evidence: "PASS: normal and upgrade journeys reached the exact six-pane 160x48 inventory with one canonical 28x46 rail and two exact chrome rows."
    - id: O2
      text: "Former toggle inputs cannot mutate pane layout."
      evidence: "PASS: literal Alt /, the delivered FIXED click, stale CLI pipe, and granted active-tiled Keybind-source toggle were inert."
    - id: O3
      text: "Native fullscreen round-trips both pane kinds without layout loss."
      evidence: "PASS: normal and upgrade journeys fullscreened exact terminal and rail IDs at 160x46 and restored every stable identity/geometry/chrome/command/fullscreen field."
    - id: O4
      text: "Entry, row interaction, permission behavior, recipient isolation, and standing-root isolation do not regress."
      evidence: "PASS: entry 14/14, main normal/upgrade, and recipient normal/hostile-inherited journeys pass; host candidate digest and log count remain empty/zero, and the inherited socket sentinel stays empty."
    - id: I1
      text: "Extra panes remain arranged after former toggle inputs in WORK."
      evidence: "PENDING CL: exact captain script is in the artifact; offline validation passed."
    - id: I2
      text: "The captain's native fullscreen journey works for ordinary and rail panes."
      evidence: "PENDING CL: exact captain script is in the artifact; offline validation passed."
---

Validation cycle 3 at `c269b31bd9ada398cebb79e839580bb4f2a6b0d8`
passes every offline criterion. The repaired main and recipient harnesses keep
all candidate state disposable, exact stable-tab/token delivery remains green,
and no candidate-attributable host record appears. I1-I2 remain for CL.

The detailed artifact contains the exact review evidence, per-AC commands,
named adversarial probes, accepted quick-review waiver, and the exact
captain-driven I1-I2 script.
