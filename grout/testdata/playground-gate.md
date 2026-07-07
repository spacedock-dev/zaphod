---
subspace: v0
gate:
  workflow: demo
  entity: playground
  entity-title: Playground handoff demo
  stage: review
  round: 1
recommendation:
  verdict: approve
  rationale: Demonstrates the TUI and the web viewer sharing one decision-log.
artifact:
  kind: draft
  path: ./playground.md
criteria:
  bar: |
    Anchored feedback survives across both review surfaces.
  acceptance:
    - id: AC-1
      text: A pin made in the TUI renders in the web viewer via the same decision-log.
      evidence: shown live in the handoff demo
---

This gate shows that `subspace-tui` and the web viewer share one decision-log.
Pins added in either surface resolve against the same artifact (`playground.md`)
and appear in both.
