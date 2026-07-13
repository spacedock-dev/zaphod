---
subspace: v0
gate:
  workflow: agent-rail-dev
  entity: managed-tab-safety-session-integration
  entity-title: Integrate managed-tab safety with tab-bound session delivery
  stage: ideation
  round: 1
recommendation:
  verdict: APPROVE
  rationale: "The plan preserves both approved V3 and BB contracts through one narrow no-ff integration, proves the combined session/focus/toggle journey in disposable state, and leaves standing Zellij configuration untouched."
artifact:
  kind: draft
  path: ../managed-tab-safety-session-integration.md
criteria:
  bar: |
    Integrate V3 managed-tab authorization with BB stable-tab session delivery
    without weakening either proof, inventing a second lifecycle mechanism, or
    touching standing Zellij state. The combined journey must be observable in
    a disposable real Zellij/tmux environment.
  acceptance:
    - id: AC-O1
      text: "The integration contains both contracts without a guessed merge."
      evidence: "Ideation recomputed a single README conflict and requires implementation to stop if the moved-main audit expands."
    - id: AC-O2
      text: "One selected-checkout managed tab delivers and acts on one session."
      evidence: "The proposed combined smoke requires a real source update, target-only render, and real row action to the unique cwd-bound terminal."
    - id: AC-O3
      text: "Alt-/ remains owned by the marked tab in the complete journey."
      evidence: "The proposed smoke sends literal keys to both the marked target and an unmarked same-WASM lookalike and compares native plus visible state."
    - id: AC-O4
      text: "The user-facing boundary is coherent and gates stay deferred."
      evidence: "The documentation resolution retains both contracts and explicitly keeps gate pooling and review in S9/QT."
---

Gate review: Integrate managed-tab safety with tab-bound session delivery — ideation
Chosen direction: make one no-ff V3 integration on current main, resolve only README, and prove the full BB+V3 journey in a disposable tmux-hosted Zellij profile.
Recommend approve.

Checklist (from `docs/agent-rail-dev/.spacedock-state/managed-tab-safety-session-integration.md` lines 232-247):
- DONE: Prove exact V3/BB merge shape
- DONE: Specify minimal integration and proof packet
- DONE: Exclude standing Zellij state

Acceptance cross-check:
- AC-O1 requires both contracts and a fresh merge audit.
- AC-O2 requires target-only delivery plus a real row action.
- AC-O3 requires literal-key target and lookalike evidence.
- AC-O4 preserves session-only scope and defers gate behavior.
- AC-I1 remains captain-live after all offline criteria pass.

No reviewer findings are pending. The recorded merge snapshot predates the three main-only Roborev policy commits, so implementation must obey the existing stop-on-expanded-conflict guard before writing.

Assessment: 3 done, 0 skipped, 0 failed.

Decision: approve to enter implementation in worktree `.worktrees/spacedock-ensign-managed-tab-safety-session-integration`; revise to bounce the concrete findings back to ideation; hold to leave the task at this gate.
