---
subspace: v0
gate:
  workflow: agent-rail-dev
  entity: zellij-transactional-retained-pane-override
  entity-title: Zellij transactional retained-pane override for safe foreign-tab docking
  stage: validation
  round: 1
recommendation:
  verdict: REJECTED
  rationale: "Exact-base RED/GREEN and inertness checks pass, but the series has no server/plugin/CLI integration or process matrix; AC-1 through AC-5 and the process evidence in AC-7 are refuted."
artifact:
  kind: draft
  path: ./zellij-transactional-retained-pane-override-validation.md
criteria:
  bar: |
    Offline acceptance requires observed pane IDs, child PIDs, geometry,
    side-effect channels, stable-tab caller results, exact-base RED/GREEN, and
    proof that Zaphod never consumes a fork.
  acceptance:
    - id: AC-1
      text: "One rail is added without changing the terminal set."
      evidence: "REFUTED: only synthetic planner IDs are compared; no panes or PIDs are created or observed."
    - id: AC-2
      text: "Unsatisfiable placement is an atomic rejection."
      evidence: "REFUTED: no tab snapshot or PTY/plugin side-effect recorder exists; the geometry unit is not run by verify.sh."
    - id: AC-3
      text: "The committed base is safe before any swap."
      evidence: "REFUTED: commit has no test and base/swaps are never installed or failure-injected."
    - id: AC-4
      text: "Placement ignores launch identity."
      evidence: "REFUTED: no cwd/argv/hold fixtures or processes are exercised."
    - id: AC-5
      text: "Callers receive the result for the requested stable tab."
      evidence: "REFUTED: result is a utility type with no event, server route, CLI command, or focus-switch test."
    - id: AC-6
      text: "The mergeable proof kit is inert."
      evidence: "PASS: protected diff, normalized metadata, installer bytes, executable inventory, and ordinary suites are unchanged without a patched runtime."
    - id: AC-7
      text: "The inert artifacts prove exact-base RED to patched GREEN."
      evidence: "REFUTED overall: exact-base phase exits pass twice and controls fail closed, but claimed pane snapshots/process matrix are static declarations rather than observations."
---

Validation recommends rejection at
`3a92ae8948c97d6f094bd260a9ff3f948077c469`. The artifact contains the two
fresh replay results, per-AC verdicts, surviving adversarial attacks, inertness
evidence, and the boundary for a later activation demo. No live demo applies
to this inert deliverable and no fork was installed, selected, shipped, or
required.
