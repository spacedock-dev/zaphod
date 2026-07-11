---
id: jk0e6wnpqcd5c40yhe9pvegh
title: Managed Zellij tab CLI entry and guarded toggle spike
status: validation
source: captain-approved managed-tab option 2, 2026-07-11
started: 2026-07-11T03:03:02Z
completed:
verdict:
score: 0.98
worktree: .worktrees/spacedock-ensign-managed-zellij-tab-cli-spike
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

## Stage Report: ideation

- DONE: Prove idempotent CLI focus/create and managed-tab-only toggle behavior in disposable Zellij 0.44.3 sessions, measuring transient Run-pane effects and persistent foreign-tab invariants.
  In session `zmspike0711`, three entry calls returned stable tab ID `2`; the final inventory held one managed tab, managed panes `5/6`, and no duplicate tab or terminal. Managed toggle changed `zaphod-docked` to `zaphod-sliver` and preserved pane IDs. Foreign toggle exited `3` and restored the original inventory.
- DONE: Prove or refute pane adoption into the existing managed tab through the smallest 0.44.3 driver/controller seam, preserving stable pane ID and PID with no unrelated mutation.
  A temporary `zellij-tile 0.44.3` controller moved terminal pane `0` from tab `0` to tab `2` through `break_panes_to_tab_with_id`; PID `18723` remained live, pane `14` and its PID remained intact, and the source tab restored pane `14` to `80x24`.
- DONE: Return an evidence-backed ownership boundary and exact evergreen-spec/development-plan changes; stop without production edits if any core invariant fails.
  No production file changed. The results require a native controller for keybindings and adoption; CLI `Run` remains a disposable proving harness.

### Live findings

- `current-tab-info --json` returned `No active tab found for current client` from the CLI link/Run client. Mapping `ZELLIJ_PANE_ID` through `list-panes --json --all` identified the invoking tab reliably.
- A tiled `Run` probe created temporary terminal pane `13` and reduced foreign pane `0` from `80x24` to `80x12`. Exit removed pane `13` and restored `80x24`. A floating Run would avoid reflow but still flicker and take focus.
- Stale binding `999` and an unbound reserved name each exited `65` without creating a tab. A nonexistent source pane returned no target and left the inventories unchanged.
- A synchronized permission denial on distinct plugin URL `adopt-controller-deny.wasm` preserved source pane `15`, PID `50156`, and foreign tab `3`. The controller closed itself on denial.
- Raw launch of a missing controller returned a plugin ID and left a Zellij error float. The launcher must preflight the exact controller artifact before sending any Zellij action.
- Sending a denial keystroke before the permission prompt appeared did not deny the request and moved the pane. The driver must never automate permission responses; it must wait for Zellij's `PermissionRequestResult`.

### Ownership boundary

- Portable CLI: derive the workspace/session binding, lock convergence, inspect tabs, validate the bound ID and reserved name, create or focus the managed view, and preflight the controller artifact.
- Zellij controller: resolve the real invoking pane/tab, enforce the managed-tab guard, steer the managed swap layout, move explicit pane IDs with `break_panes_to_tab_with_id`, and surface permission/result status.
- Portable hub/dock: retain item identity, providers, review routing, rendering, and workspace policy. None belongs in the controller.

### Evergreen documentation changes

- Rename `docs/superpowers/specs/2026-07-10-zaphod-minimum-list-workspace-design.md` into one canonical evergreen architecture document directly under `docs/`.
- Replace current-tab `ensure_dock` with `ensure_managed_view(binding)`: one managed Zellij tab or tmux window per binding; never retrofit a foreign view.
- Define `Alt Shift z` as idempotent create-or-focus. Define `Alt /` as a controller request that changes layouts only when the invoking tab ID equals the recorded managed ID.
- Define explicit pane adoption as an optional driver capability. Zellij uses `break_panes_to_tab_with_id`; the action preserves pane/process identity and may close an emptied source tab.
- Update every diagram to show foreign views beside, rather than inside, the managed view. Remove the retained-layout transaction from the critical path.
- Reorder `docs/plan-agent-rail.md`: managed-view binding and the minimal controller precede hub/dock/providers; park j5, eh, fw, 4d, and the upstream transactional request outside the release path.

### Summary

Managed-view convergence and native pane adoption work on Zellij 0.44.3. Option 2 proves the lifecycle but visibly disturbs the current tab, so the durable design should bind both keys to a minimal native controller while retaining CLI create/focus for external launch and testing.

## Stage Report: implementation

- DONE: Rename the approved dated minimum-workspace design into one evergreen docs/ architecture spec and update every repository reference without leaving a competing copy.
  Commit `ee4765c` preserves a 65% Git rename to `docs/zaphod-workspace-architecture.md`; the dated path is absent and a repository-wide search found no stale references.
- DONE: Replace foreign/current-view docking with the spike-proved managed-view lifecycle, stable-ID routing, production controller boundary, managed-only Alt-/, and explicit identity-preserving pane adoption.
  The spec records the option-2 Run limitation, invoking-pane lookup, stable-ID guard, managed-only toggle, explicit pane/PID-preserving adoption, failure cases, and non-automated consent.
- DONE: Reconcile the development plan with the new driver/hub/provider delivery order and clearly distinguish the shipped prototype from the target architecture; verify prose, diagrams, and links for contradictions.
  `docs/plan-agent-rail.md` now separates the prototype record from the managed-view delivery order; `git diff --check`, path checks, and stale-term/reference searches passed.
- SKIPPED: Red/green product tests.
  This dispatch explicitly allowed documentation only; it changed no production source, tests, installed layout, or multiplexer state.

### Summary

The evergreen architecture now makes one managed tab or window the workspace/session binding invariant. A thin Zellij controller owns native key and pane operations, while the portable launcher, hub, dock, and providers retain their independent boundaries; foreign-view retrofit work is no longer on the release path.
