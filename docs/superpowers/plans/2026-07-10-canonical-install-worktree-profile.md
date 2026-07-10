# Canonical Install and Worktree Profile Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make global Zaphod installation primary-checkout-only and provide a disposable, candidate-coherent Zellij profile for linked-worktree validation.

**Architecture:** `scripts/zellij-layout-lib.sh` owns physical-root, URL, identity, and rendering helpers. `install.sh` applies a fail-closed preflight/render/parse/atomic-rename/postflight transaction; `scripts/zellij-worktree-test-profile.sh` builds and launches a temp-root profile without calling the installer. Process-level shell regressions exercise actual scripts, Zellij 0.44.3, live session state, cleanup, and outside-file hashes.

**Tech Stack:** Bash 3.2-compatible shell, Git worktrees, Zellij 0.44.3, KDL layouts, SHA-256, Rust/Cargo, Go.

## Global Constraints

- All global install writes come from the primary checkout release WASM and stop before mutation on any identity mismatch.
- Worktree validation uses disposable config, layout, data, and session state and never copies, rewrites, or restores global files.
- Zellij-dependent tests require exactly 0.44.3 and fail loudly when it is unavailable.
- Runtime j5 layout-transform behavior and pz confirmation behavior remain out of scope.
- Each behavior follows observed RED, minimal GREEN, and its own commit.

---

### Task 1: Primary-checkout install guard

**Files:**
- Create: `tests/zellij-install-profile-test.sh`
- Create: `scripts/zellij-layout-lib.sh`
- Modify: `install.sh`

**Interfaces:**
- Consumes: Git absolute common directory and the invoking checkout's physical root.
- Produces: `zaphod_primary_checkout_root`, `zaphod_canonical_file_url`, and `zaphod_render_layout` shell functions; linked invocations exit before destination mutation.

- [ ] Add a process test that creates a temporary clone and linked worktree, builds/copies fixture WASMs, seeds config/layout sentinels, and proves today's linked `install.sh` overwrites the layout.
- [ ] Run `./tests/zellij-install-profile-test.sh linked-install` and record the predicted red output/exit.
- [ ] Add the smallest physical primary-root comparison and reusable renderer; refuse before destination creation when roots differ.
- [ ] Re-run the focused test and its primary-clone success control.
- [ ] Commit the guard, renderer, and regression.

### Task 2: Identity preflight and atomic postflight

**Files:**
- Modify: `tests/zellij-install-profile-test.sh`
- Modify: `scripts/zellij-layout-lib.sh`
- Modify: `install.sh`

**Interfaces:**
- Consumes: effective `config.kdl`, canonical primary `file:` URL, `rail "1"`, rendered temp layout.
- Produces: `zaphod_validate_message_plugin_identity`; atomic rename with restoration/removal on failed postflight.

- [ ] Add table-driven foreign-URL, mixed-URL, missing-keybind, missing-rail, and coherent cases that preserve a sentinel on rejection and replace it only when coherent.
- [ ] Run the focused identity test and record today's predicted red acceptance/failure.
- [ ] Implement config identity parsing, destination-local temporary rendering, `zellij setup --check`, disposable layout parsing, atomic rename, and postflight rollback.
- [ ] Run the identity cases and the full shell suite green.
- [ ] Commit identity validation and transactional install behavior.

### Task 3: Disposable worktree-test profile

**Files:**
- Modify: `tests/zellij-install-profile-test.sh`
- Create: `scripts/zellij-worktree-test-profile.sh`

**Interfaces:**
- Consumes: `--cwd PATH`, candidate checkout `./build.sh`, shared renderer, global file snapshots.
- Produces: temp `config/config.kdl`, `config/layouts/zaphod.kdl`, `config/layouts/explicit-cwd.kdl`, `data/`, unique attached session, printed candidate metadata/inspection commands, trap cleanup.

- [ ] Add process tests that fail because the profile command is absent and cover normal plus TERM/INT cleanup, live dump identity, containment, and unchanged outside-file hashes.
- [ ] Run the focused profile tests and record the missing-command red output.
- [ ] Implement argument validation, candidate build, temp-root rendering, launch metadata, attached Zellij invocation, and idempotent cleanup traps.
- [ ] Run normal and signal cases, then the full shell suite green.
- [ ] Commit the profile and lifecycle tests.

### Task 4: Operator and delivery documentation

**Files:**
- Modify: `README.md`
- Modify: `docs/docking-approach.md`
- Modify: `docs/agent-rail-dev/README.md`
- Modify or create after first-officer clarification: `docs/prd-agent-rail.md`

**Interfaces:**
- Consumes: executable commands and verified delivery dependency audit from the approved entity.
- Produces: canonical-install/candidate-profile operator paths, live-state oracle, global-hash discipline, architecture/data-flow diagram, current-main versus yb/pz boundary, and pz confirmation/fail-loud gates.

- [ ] Replace ambiguous build/install examples with the primary-only global path and explicit unmerged-worktree profile path.
- [ ] Add docking provenance/identity/oracle/hash guidance and correct the workflow dogfood sentence.
- [ ] Add the approved delivery order and architecture/data-flow section to the authoritative PRD path once clarified.
- [ ] Rehearse documented commands from the linked worktree and review the diff against AC-1 through AC-5.
- [ ] Commit documentation as one operator-contract change.

### Task 5: Final verification and report

**Files:**
- Modify: `/Users/clkao/git/zaphod/docs/agent-rail-dev/.spacedock-state/canonical-install-and-worktree-test-profile.md` (body only, separate state checkout commit)

**Interfaces:**
- Consumes: all focused red/green evidence and commit SHAs.
- Produces: implementation stage report and completion signal.

- [ ] Run the shell suite, `cargo test`, `cargo check --tests`, `go test ./...`, and `go vet ./...` with exact counts/status recorded.
- [ ] Confirm the code worktree is clean and all production commits are on the assigned branch.
- [ ] Append the implementation stage report without changing frontmatter and commit only that entity path in the state checkout.
- [ ] Send the exact Codex Ensign completion signal.
