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

### Feedback Cycles

- **Cycle 1 — validation → implementation (2026-07-11): REJECTED.** The evergreen spec, rename history, managed-view contract, and delivery order passed, but `docs/docking-approach.md` still presents current-tab retrofit as the adopted architecture and `docs/plan-agent-rail.md` repeats that stale label. Preserve the docking document as historical prototype evidence, mark it superseded by `docs/zaphod-workspace-architecture.md` for product direction, remove stale “adopted architecture” wording, and rerun the repository-wide contradiction/reference audit. No production or behavioral change is requested.

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

## Stage Report: implementation (cycle 2)

- DONE: Mark `docs/docking-approach.md` as historical evidence for the shipped Zellij WASM prototype without removing its retrofit findings.
  Commit `4d2ca74` adds a prominent supersession notice, links `docs/zaphod-workspace-architecture.md`, and renames stale “adopted architecture” labels to “shipped prototype architecture.”
- DONE: Remove the stale target-architecture label from `docs/plan-agent-rail.md`.
  The plan now calls the v3.12 dock container a historical prototype while retaining its target delivery order and evergreen-spec link.
- DONE: Re-run repository-wide contradiction and formatting checks.
  `git diff --check` passed; searches found no old dated-spec path, old spec title, approved-design status, `adopted architecture`, or current-tab target claim outside the evergreen non-goal.
- DONE: Leave the worktree clean without production changes.
  `git status --short --branch` reported a clean documentation branch after `4d2ca74`; only the two requested docs changed.

### Summary

Readers can now distinguish the shipped current-tab retrofit prototype from the managed-view product architecture at the start of either planning path. The prototype’s live evidence remains available as historical engineering context.

## Stage Report: validation

- DONE: Validate the implementation at its dispatched raw commit identity without trusting the implementation report.
  The clean worktree HEAD was `ee4765cca737e377c00d573689d0fe37aec42bff`; `git diff --check ee4765c^ ee4765c` passed.
- DONE: AC-1 — Idempotent managed entry.
  The spec's one-managed-view, stable-ID create-or-focus claims match jk's three calls returning tab ID `2` with no duplicate tab or terminal.
- DONE: AC-2 — Foreign-tab toggle guard.
  The spec requires a stable-ID guard and no foreign-view effect, while lines 283-299 explicitly demote the focus-taking, 80x24-to-80x12 CLI `Run` pane to a test harness.
- DONE: AC-3 — Pane adoption preserves identity.
  The spec limits adoption to an explicit native move and accurately records pane `0` moving to tab `2` with its PID and the unrelated terminal preserved.
- DONE: AC-4 — Ownership boundary.
  Launcher convergence/preflight, controller-native tab and pane operations, and hub/provider/dock responsibilities match the spike inventory without moving product state into WASM.
- DONE: AC-5 — Failure behavior.
  Stale ID, duplicate reserved name, missing pane/controller, refusal, preflight, and non-automated `PermissionRequestResult` claims are bounded by jk's durable negative-case evidence.
- FAILED: Verify the dated design became exactly one evergreen docs/ spec with preserved history, valid references, and no stale competing architecture path.
  Rename/history and references pass (`R065`, prior commit `c66400d`, old path absent), but `docs/docking-approach.md:3,237-247,279-299` still calls current-tab retrofit the adopted/shipped path; `docs/plan-agent-rail.md:6` reinforces that stale label.
- DONE: Cross-check every managed-view, keybinding, identity, pane-adoption, permission, and failure claim against the durable spike evidence; reject overclaims or hidden foreign-tab mutation.
  The new evergreen spec passes line-by-line: both keys target the controller, CLI `Run` is prototype-only, stable IDs fail closed, and pane adoption is explicit and permission-gated.
- DONE: Audit the revised delivery plan and diagrams for ownership/order consistency, prototype-versus-target clarity, and removal of j5/4d foreign-retrofit work from the release path.
  Architecture flows preserve launcher/driver/hub/dock/provider ownership; plan steps 1-8 follow that order and park `j5`, `eh`, `fw`, and `4d` outside the release path.
- DONE: Run an adversarial refutation audit on a disposable checkout.
  Attacks covered stale-reference false negatives and synonyms, rename false positives/history, caller/reference impact, and semantic drift against the parent spec; executable panic/indexing paths were inapplicable to this docs-only diff.
- SKIPPED: Re-run the live Zellij drill or prepare a new interactive demo.
  The dispatch says no live rerun is required unless a claim lacks durable evidence; all revised-spec behavior claims had jk evidence, and the surviving defect is static documentation state.

### Summary

Commit `ee4765c` correctly defines the managed-view target, preserves the dated spec's history, and removes CLI `Run` and foreign-tab mutation from the product path. Validation recommends rejection because `docs/docking-approach.md` still presents the destructive foreign-tab retrofit as the adopted architecture and the revised plan repeats that label. Mark that document explicitly historical/superseded and remove the current “adopted architecture” wording before revalidation.

## Stage Report: validation (cycle 2)

- DONE: Independently inspect the exact two-doc correction after `ee4765c` at raw commit `4d2ca749a4493fbdb34a42801de88107e25f2fa9`.
  Only `docs/docking-approach.md` and `docs/plan-agent-rail.md` changed; the worktree was clean and `git diff --check ee4765c..4d2ca74` passed.
- DONE: Confirm `docs/docking-approach.md` is now unmistakably historical prototype evidence and preserves useful retrofit findings without presenting that file itself as product direction.
  Its opening notice says “Historical prototype record,” “not the product architecture,” links the evergreen spec, and renames its internal heading to “Shipped prototype architecture.”
- DONE: Confirm `docs/plan-agent-rail.md` no longer calls the docking document adopted architecture.
  Line 6 now labels the v3.12 dock container “historical prototype,” while the managed-view target order and evergreen link remain intact.
- DONE: Re-run old-path/title/status, link/path, formatting, rename, and history checks.
  The old dated path/title/status are absent, link targets exist, the original move remains `R065`, and `git log --follow` reaches `c66400d` through `ee4765c`.
- FAILED: Complete the focused repository-wide contradiction and stale target-claim audit.
  `SPEC.md:157-159` still calls the every-tab sidebar model “the adopted architecture,” and `SPEC.md:367-394` recommends first-toggle retrofit as “v2 from scratch,” including another “Adopted architecture” citation at line 387.
- DONE: Confirm the prior managed-view and spike acceptance evidence remains unaffected.
  The evergreen architecture blob is identical at `ee4765c` and `4d2ca74`; AC-1 through AC-5 and jk's stable-ID, guard, adoption, permission, and failure evidence therefore remain unchanged.
- SKIPPED: Re-run live Zellij.
  The correction adds no behavior claim and the rejection is a static competing-document contradiction, so the durable live evidence remains sufficient.

### Summary

Recommendation: **REJECTED**. The two-doc correction is accurate and complete in isolation, but root `SPEC.md` still presents the foreign-tab retrofit as adopted and future-facing architecture, contradicting the evergreen managed-view spec. Mark `SPEC.md` as historical prototype evidence and/or rewrite its two target claims to point readers to `docs/zaphod-workspace-architecture.md`, then rerun the repository-wide search.
