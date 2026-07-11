---
id: jk0e6wnpqcd5c40yhe9pvegh
title: Managed Zellij tab CLI entry and guarded toggle spike
status: ideation
source: captain-approved managed-tab option 2, 2026-07-11
started: 2026-07-11T03:03:02Z
completed:
verdict:
score: 0.98
worktree:
issue:
pr:
mod-block:
---

## Problem

The approved minimum-workspace architecture still says Zaphod inserts a dock into the current foreign Zellij tab. j5/4d proved that retained-layout retrofit is destructive or duplicative because Zellij 0.44.3 lacks a transactional retained-pane operation and original typed pane identity. The selected replacement is one Zaphod-managed tab per workspace/session binding, entered idempotently with `Alt Shift z`. `Alt /` must have no layout effect outside that managed tab. A user who wants an existing pane managed moves that pane into the managed tab instead of asking Zaphod to reconstruct the foreign tab.

Option 2 uses a CLI helper invoked by a Zellij `Run` keybinding as the initial entry/toggle mechanism. Zellij 0.44.3 CLI exposes stable tab IDs, `current-tab-info`, `list-tabs`, `new-tab --layout`, and `go-to-tab-name`, but no move-to-existing-tab command. Its plugin API exposes `break_panes_to_tab_with_id`, so the spike must identify the smallest controller seam needed for pane adoption without moving the dock model or provider logic back into WASM.

## Proposed approach

In disposable Zellij 0.44.3 sessions only, build temporary fixtures for:

- `Alt Shift z` → `zaphod view`: discover the workspace/session binding, focus its recorded managed tab ID when live, otherwise create a uniquely named tab from a managed layout and record the returned stable tab ID;
- `Alt /` → `zaphod toggle`: read `current-tab-info --json`, compare the active stable tab ID to the binding, and call the managed tab's swap-layout action only on a match;
- `zaphod adopt-pane PANE_ID`: use the smallest Zellij-driver controller surface around `break_panes_to_tab_with_id` to move the selected terminal into the managed tab while preserving its pane ID and process.

The `Run` keybinding opens a transient command pane. The spike must measure its visible and structural effects and decide whether it is acceptable for the first launcher or only as a proving harness. No standing global config, installed layout, WORK session, production source, or external fork may be changed.

## Acceptance criteria

### Offline/live spike

**AC-1 — Idempotent managed entry.** From a disposable session with multiple foreign tabs, the first simulated `Alt Shift z` creates exactly one managed tab from the supplied layout; subsequent invocations focus the same stable tab ID and create no additional pane or tab.
Verified by: before/after `list-tabs --json`, `list-panes --json -a`, returned tab IDs, and managed-layout identity across at least three invocations.

**AC-2 — Foreign-tab toggle guard.** Invoking the option-2 toggle helper from every foreign tab performs no persistent layout, pane, focus, or process change there. Invoking it from the managed tab advances exactly one known managed swap-layout state.
Verified by: current-tab stable ID checks plus before/after pane IDs, geometry, active swap-layout name, PIDs, and a bounded observation of any transient `Run` pane/flicker.

**AC-3 — Pane adoption preserves identity.** Moving one selected terminal from a foreign tab into the managed tab preserves its terminal pane ID and process PID, removes no unrelated pane, creates no replacement terminal, and leaves the source tab otherwise unchanged.
Verified by: a disposable controller using Zellij 0.44.3 `break_panes_to_tab_with_id`, with before/after pane/PID/tab observations from outside the controller.

**AC-4 — Ownership boundary.** The spike identifies exactly which behavior remains in the portable launcher/CLI and which minimal native operation belongs to the Zellij driver/controller. No session/review item model, provider parsing, hub routing, or dock rendering enters the controller.
Verified by: the proposed evergreen spec diff and source-boundary inventory cite the live results, including any option-2 limitation.

**AC-5 — Failure behavior.** Stale tab binding, duplicate reserved tab name, closed source pane, controller unavailable, and permission refusal fail visibly without touching foreign tab layouts or silently creating a second managed view.
Verified by: disposable negative cases with exact exit/status evidence and final pane/tab snapshots.

## Test plan

First prove the CLI-only focus/create and toggle guard with temporary scripts and a disposable profile. Then exercise pane movement with the smallest temporary controller against the same session. Repeat entry three times, toggle from managed and foreign tabs, move one long-running canary terminal, and compare stable tab IDs, pane IDs, PIDs, geometry, and swap-layout state. Treat persistent foreign-tab mutation, duplicate managed tabs, pane/PID replacement, or a hidden dependency on global config as design-invalidating.

## Documentation outcome after the spike

If the mechanism passes, implementation updates the minimum-list workspace design to make a managed tab/window the canonical view, replaces per-current-view `ensure_dock` semantics with managed-view convergence, documents `Alt Shift z`, scopes `Alt /` to the managed view, and defines pane adoption as a driver capability. Move the approved design from `docs/superpowers/specs/` to an evergreen document directly under `docs/`, updating diagrams and the development plan. Preserve history with a Git rename rather than duplicate competing specs.

## Out of scope

Building the portable dock or workspace hub; tmux implementation beyond recording the analogous managed-window contract; editing production Zaphod code during the spike; installing a Zellij fork; reviving foreign-tab retained override; moving panes automatically without an explicit user action.
