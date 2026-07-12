# Fail-Closed Alt Toggle Implementation Plan

> **For Claude:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `Alt /` toggle only a layout-initialized Zaphod rail and leave every foreign tab unchanged.

**Architecture:** The durable config fallback is `NoOp`, so an uninitialized session cannot launch a plugin. An active tiled rail temporarily reconfigures only its current client to route `Alt /` through `MessagePluginId <plugin-id>`; Zellij 0.44.3 dispatches that action directly to the existing plugin and never calls its launch path. The sidebar itself refuses every pipe unless it is the active tiled rail.

**Tech Stack:** Rust/WASM (`zellij-tile` 0.44.3), Zellij KDL, Bash/AWK activation tests, tmux-hosted Zellij smoke documentation.

---

## Chunk 1: Fail-closed route and rail decision

### Task 1: Persisted activation fallback

**Files:**

- Modify: `tests/zellij-new-tab-test.sh`
- Modify: `scripts/zellij-config-activate.awk`

- [x] **Step 1: Write the failing shell assertion** that each Zaphod-owned `Alt /` binding is exactly `NoOp` after activation, while `Alt Shift z` remains `NewTab { layout "zaphod"; }`.
- [x] **Step 2: Run** `./tests/zellij-new-tab-test.sh` and verify it fails because activation preserves `MessagePlugin` for `Alt /`.
- [x] **Step 3: Implement the smallest transformer change** that removes the existing Zaphod `Alt /` binding and emits `bind "Alt /" { NoOp; }` in only those scopes, retaining URL/rail routing for `Alt .` and native `Alt Shift z`.
- [x] **Step 4: Re-run** `./tests/zellij-new-tab-test.sh` and verify the focused activation assertion is green.

### Task 2: Rail-only toggle decision and runtime route

**Files:**

- Modify: `src/main.rs`

- [x] **Step 1: Write failing Rust tests** that a sidebar instance observing a sidebar-less active foreign tab returns `ToggleAction::Ignore`, that an active tiled resident keeps its existing steering behavior, and that the dynamic partial KDL binds `Alt /` to that sidebar's `MessagePluginId` with `name "toggle"`.
- [x] **Step 2: Run** the targeted `cargo test` filters and verify the absent-resident and KDL assertions fail for the old retrofit route/missing helper.
- [x] **Step 3: Implement minimally:** remove the retrofit action and bootstrap path; add a pure runtime-KDL helper; request `PermissionType::Reconfigure`; once the sidebar is the active tiled rail, call `reconfigure(kdl, false)`; preserve resident swap steering.
- [x] **Step 4: Re-run** the targeted Rust tests and verify they pass.
- [x] **Step 5: Run** `cargo test` and `cargo check --tests` to catch integration/type errors.

### Task 3: Operator contract and live smoke boundary

**Files:**

- Modify: `README.md`
- Modify: `docs/docking-approach.md`
- Modify: `docs/zellij-tmux-smoke-harness.md`

- [x] **Step 1: Replace** all claims that first `Alt /` retrofits a tab with the `Alt Shift z` fresh-tab entry path and rail-only `Alt /` behavior.
- [x] **Step 2: Document** that Reconfigure permission is needed for the temporary, current-client `MessagePluginId` route; persistent config remains `NoOp`, and a newly attached client safely no-ops until its rail routes itself.
- [x] **Step 3: Add smoke assertions** for: foreign-tab `Alt /` leaves pane/tab/focus state unchanged; initialized rail `Alt /` changes only its known swap state; no custom PTY.
- [x] **Step 4: Re-run** the focused shell suite, Rust suite, and regression bridge tests.

### Task 4: Commit and handoff

**Files:**

- Verify: `src/main.rs`, `scripts/zellij-config-activate.awk`, `tests/zellij-new-tab-test.sh`, `README.md`, `docs/zellij-tmux-smoke-harness.md`

- [x] **Step 1: Inspect** `git diff --check` and `git status --short`; confirm no 7h, 4d, or workflow-state path changed.
- [ ] **Step 2: Commit** the tested code and documentation as one separate fail-closed-toggle commit.
- [ ] **Step 3: Report** the red/green commands, native semantic proof, commit SHA, and the per-client safe-fail limitation.
