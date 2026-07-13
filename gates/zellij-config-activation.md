---
subspace: v0
gate:
  workflow: agent-rail-dev
  entity: zellij-config-activation
  entity-title: Activation preserves valid Zellij KDL
  stage: validation
  round: 1
recommendation:
  verdict: PENDING-DEMO
  rationale: "Offline AC-1 through AC-4 independently pass at exact head e238f7a; refutation attacks found no surviving defect; the captain direct-script demo remains pending and must not use Alt keys."
artifact:
  kind: draft
  path: ./zellij-config-activation-validation.md
criteria:
  bar: |
    The selected-checkout command must create exactly one selected-WASM tab at
    its returned stable ID while valid standing config/layout bytes survive
    success, setup/action failure, and TERM. Validation must use an isolated
    Zellij-in-tmux harness and a throwaway-checkout refutation audit.
  acceptance:
    - id: AC-1
      text: "Selected-checkout fresh tab is the end value."
      evidence: "PASS: focused fake-Zellij and real tmux/Zellij state observed one inline selected-WASM tab at the returned stable ID."
    - id: AC-2
      text: "Valid unrelated KDL is preserved, not transformed."
      evidence: "PASS: native setup accepted quoted-brace/comment/fixed-shortcut KDL; isolated and standing config/layout hashes remained identical."
    - id: AC-3
      text: "Failure and interruption cannot mutate standing KDL."
      evidence: "PASS: setup failure, new-tab failure, and blocked TERM preserved hashes and left no rendered or config-directory temporary artifacts."
    - id: AC-4
      text: "The direct path does not depend on the AWK transformer or a pre-existing Zaphod key route."
      evidence: "PASS: fixture omitted both and still created the selected inline tab while the unrelated shortcut stayed byte-identical."
---

Offline validation recommends approval at
`e238f7a90a62b068772742ccd5ac4a0fc6071727`. Roborev parent `189` covers the
exact merge-base range and is a complete passing `code_completion` panel.
Focused shell 6/6, real isolated tmux/Zellij, and Rust 135/135 passed.

Three throwaway-checkout attacks proved stable-ID discrimination: wrong-tab
same-URL state failed, correct target plus a foreign duplicate passed, and two
same-tab matches failed closed. Cleanup, caller-impact, and semantic-drift
audits found no surviving defect.

The captain demo uses only `scripts/zellij-new-tab.sh`, then checks native
pane/tab state and before/after standing hashes. It must not press `Alt /` or
`Alt Shift z`; global key policy and the deferred complex-layout chrome issue
are outside this gate. Exact commands are in the artifact.
