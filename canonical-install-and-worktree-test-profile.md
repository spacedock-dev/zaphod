---
title: Canonical install and isolated worktree test profile
status: implementation
source: finding — runtime audit found layout/keybind plugin identity split, 2026-07-10
started: 2026-07-10T12:38:46Z
completed:
verdict:
score: 0.95
worktree: .worktrees/spacedock-ensign-canonical-install-and-worktree-test-profile
issue:
pr:
mod-block:
id: v9hwpzcc1cteqfdgta461f5f
---

## Problem

The global zaphod layout can be installed from an unmerged Git worktree while
the user's keybinds still target the canonical checkout. Zellij keys plugin
instances by URL plus configuration, so this produces distinct resident and
message-launched plugin identities and invalidates live testing. The workspace
needs one verifiable canonical install and a separate, disposable way to build
and exercise an unmerged worktree without mutating `~/.config/zellij`.

This task owns install provenance, identity verification, an isolated
worktree-test profile, regression coverage for those boundaries, and the
operator documentation/j5 drill that uses the profile. It does not change the
runtime layout transform (j5) or investigate chrome corruption and leaked
instances (eh).

The incident is present on this machine: the installed layout names the
`spacedock-ensign-grout-sse-daemon` worktree WASM, while every Zaphod
`MessagePlugin` keybind in the global config names the primary checkout WASM.
The primary checkout artifact is currently absent. A layout file and a
keybind file that each parse successfully can therefore still launch different
plugin identities.

## Proposed approach

### 1. Make global installation a primary-checkout-only operation

Keep `install.sh` as the only global installer, but make it fail closed before
its first write:

1. Resolve the script checkout with physical paths. Resolve the primary
   checkout independently from Git's absolute common directory (the parent of
   the primary checkout's `.git` directory). Refuse when they differ. A linked
   worktree cannot opt out through `ZELLIJ_CONFIG_DIR`; worktree testing uses
   the profile below.
2. Require the primary checkout release WASM to exist and canonicalize its
   `file:` URL.
3. Preflight the effective `config.kdl`: every Zaphod `MessagePlugin` entry
   must use that URL and carry `rail "1"`. Missing or mixed Zaphod keybinds are
   an error. The installer reports the offending URLs and writes nothing; it
   never rewrites the user's config.
4. Render `layouts/zaphod.kdl` to a temporary file in the destination, validate
   the effective config with `zellij setup --check`, parse the layout through a
   disposable Zellij session, and atomically rename the layout. Run the same
   identity check after the rename. A failed postflight atomically restores the
   previous layout bytes (or removes a newly-created file), returns nonzero,
   and never claims a valid install.

The one-time cleanup is explicit: build main, repoint the existing global
Zaphod keybinds to main, and run the canonical installer. Recurrence prevention
is the refusal boundary and the pre/postflight; cleanup itself is not the
feature.

`install.sh` currently contains no reusable pure function. Implementation
should extract small shell functions for primary-root discovery, URL
canonicalization, keybind identity validation, and layout rendering. Both the
installer and profile reuse only the renderer; the profile does not call or
bypass the global installer.

### 2. Add a disposable worktree test profile

Add one operator command,
`./scripts/zellij-worktree-test-profile.sh --cwd PATH`, that is valid from the
primary checkout or any linked worktree. It runs that checkout's `./build.sh`,
then creates a `mktemp` root containing:

- `config/config.kdl`, with one `Alt /` `MessagePlugin` binding to the candidate
  URL and configuration `rail "1"`;
- `config/layouts/zaphod.kdl`, rendered from the production template with that
  same candidate URL;
- `config/layouts/explicit-cwd.kdl`, containing one terminal leaf with an
  explicit caller-supplied `cwd` and no plugin; and
- `data/`, so plugin cache and permission state are disposable too.

The command launches a uniquely named attached session with `--config-dir`,
`--data-dir`, and the explicit-cwd fixture. It prints the profile root, session
name, candidate commit, candidate URL, and inspection commands. A trap kills
the session and removes the profile on exit or interruption. It snapshots the
existence and SHA-256 of global `config.kdl` and `layouts/zaphod.kdl` before
launch, then fails cleanup if either byte stream changed. It never copies,
rewrites, or restores global files.

The profile includes `zaphod.kdl` even though the j5 drill starts from
`explicit-cwd.kdl`. This permits a resident-identity control: start a second
disposable session from the profile's Zaphod layout, press the profile's real
`Alt /`, and observe one resident plugin before and after. The j5 fixture then
proves the retrofit path: after its real `Alt /`, the newly tiled rail URL must
also equal the candidate URL.

The independent oracle is Zellij's live state: `action list-panes --json -a
-g -t` supplies pane IDs, counts, kinds, and geometry, while `action
dump-layout` supplies the plugin URL and chrome structure that the server
actually loaded. Git's `worktree list --porcelain` independently identifies
the primary checkout for the refusal test, and before/after SHA-256 snapshots
come from the files outside the profile. The renderer's own output or a grep of
the script cannot satisfy an acceptance criterion.

### Alternatives considered

1. **Disposable isolated profile (chosen).** Both identity-bearing files,
   cache, permissions, session, and cleanup live under one temporary root. It
   tests the candidate without exposing live global sessions to candidate
   state.
2. **Versioned global candidate layouts.** A name such as
   `zaphod-candidate.kdl` avoids overwriting `zaphod.kdl`, but a real `Alt /`
   still needs a global keybind change or another config. It also leaves stale
   layouts, permissions, and ambiguous provenance in the operator's standing
   environment. Once the config is isolated, this becomes the chosen profile
   with extra global residue.
3. **Temporary overwrite and restore.** Backing up global config/layout,
   writing candidate paths, and restoring after the run has a crash and signal
   window in which live sessions see candidate identity. Concurrent edits can
   also be overwritten by the restore. This cannot meet the requirement that
   global bytes never change.

## Acceptance criteria

### Offline

**AC-1 — A linked worktree cannot change a global Zaphod install.** Running
the actual `install.sh` from a linked worktree exits nonzero before any write,
even when the worktree has a built artifact and `ZELLIJ_CONFIG_DIR` names a
writable destination. The destination config and layout retain their original
existence and bytes.

Verified by: a shell regression creates a temporary primary clone plus linked
worktree, seeds destination sentinels, runs the linked copy of `install.sh`,
and compares exit status and before/after SHA-256. The expected primary path
comes from `git worktree list --porcelain`, outside the installer. The paired
primary-clone control succeeds against the same fixture.

**AC-2 — Canonical installation refuses split URL or configuration identity.**
The primary installer writes no layout when a Zaphod keybind names another
checkout, omits `rail "1"`, or mixes candidate URLs. With coherent keybinds,
the installed layout names only the primary-checkout URL and the command
succeeds.

Verified by: table-driven temporary-config cases preserve a sentinel layout on
each rejected input and replace it only for the coherent case; `zellij setup
--check` validates KDL. The end-value oracle is the live resident control in
AC-5, not a substring assertion over generated KDL.

**AC-3 — The worktree profile is self-contained and preserves global bytes.**
For a candidate linked worktree, both profile identity-bearing files resolve
to that candidate's physical WASM and `rail "1"`; the profile config, layout,
data, and session live below its temporary root. Global config and layout are
byte-identical after normal exit and signal-driven cleanup.

Verified by: a process-level test snapshots the real or sentinel global files,
runs create/verify/cleanup, and compares existence plus SHA-256 afterward. It
starts Zellij with the emitted argv and uses live `dump-layout` to assert the
resident URL. The expected URL is derived independently from the test
worktree's physical path.

**AC-4 — Regression coverage is red first and rejects false confidence.** The
linked-install test fails against today's installer because it overwrites the
sentinel with a worktree URL. The profile test fails before the profile exists.
The committed tests fail loudly when Zellij 0.44.3 is unavailable; no riskiest
drill may silently skip and leave the suite green.

Verified by: implementation records the exact red exit/output before each
minimal fix, then runs the shell suite plus existing Rust and Go suites. A
fixture-only KDL grep does not count as the red or the green proof.

**AC-5 — Operator and delivery documentation states executable boundaries.**
README and docking documentation distinguish primary global install from
candidate profile testing, document setup/run/inspect/cleanup commands, name
the live identity oracle, and prohibit worktree `install.sh`. The PRD records
the verified merge sequence and gates below.

Verified by: review the doc diff against AC-1 through AC-4's executable results
and the dependency audit below; prose matching cannot prove this AC alone.

### Interactive

**AC-6 — One profile has one candidate identity in a real session.** In the
resident control, CL starts the profile's Zaphod layout and presses real
`Alt /` once. Exactly one sidebar plugin exists before and after, and every
sidebar URL in the live dump equals the tested checkout's URL.

Verified by: CL's attached-session observation plus before/after
`list-panes --json -a` and `dump-layout`. A second plugin ID or any URL outside
the tested checkout fails the gate.

**AC-7 — The profile supports the j5 red-baseline/candidate drill.** Starting
from `explicit-cwd.kdl`, before the keypress there is exactly one terminal ID,
zero rails, and canonical top/bottom chrome. After one real `Alt /`, the j5
candidate retains that same terminal ID, has no second terminal, has exactly
one rail at the candidate URL, and preserves canonical chrome. Current main,
run through the same profile and commands, is recorded red by producing the
known extra terminal.

Verified by: CL drives the attached session; captured before/after JSON and
layout dumps are compared by IDs and counts, not visual similarity. The
profile/global byte snapshots are unchanged after both baseline and candidate
runs, and both sessions/profile roots are absent after cleanup.

## Test plan

1. **Riskiest mechanism first — spike needed.** Prove that a single attached
   Zellij 0.44.3 process launched with temporary `--config-dir` and
   `--data-dir` loads both the resident layout and real `Alt /` keybind from
   that root, reports one candidate URL through live state, and leaves global
   config/layout SHA-256 unchanged after teardown. This combined mechanism has
   not yet been exercised; CLI help proves the flags exist but not the
   end-to-end isolation claim. Abort implementation if live state shows a
   global URL or a changed global hash.
2. Add the linked-worktree refusal regression first. Capture today's red:
   linked `install.sh` replaces the sentinel layout. Add only the primary-root
   guard, then prove linked refusal and primary success.
3. Add table-driven identity-preflight cases: coherent URL/config succeeds;
   foreign URL, mixed URLs, missing keybind, and missing `rail "1"` all fail
   before the destination changes. Run `zellij setup --check` on accepted
   material.
4. Add profile lifecycle tests for normal exit and TERM/INT cleanup. Assert
   unique session deletion, profile deletion, data-dir containment, and
   unchanged global file existence and SHA-256.
5. Run the resident control through live Zellij state: one plugin before and
   after one real keypress, all URLs candidate-only.
6. At j5's gate, build current main and j5 candidate separately. Use the same
   profile command, explicit cwd, attached-terminal key dispatch, JSON/dump
   capture, and cleanup. Record main red, then require candidate green for the
   exact terminal ID/count, rail count/URL, chrome, and global-byte assertions.
7. Run all existing Rust and Go suites to catch unrelated regressions, then
   rehearse the README commands from a fresh linked worktree.

No real-dump parser unit fixture is added by this task. If implementation adds
one, it must use Zellij's real single-line plugin-node shape; process-level
state remains the authoritative proof.

## Documentation diff

**README — replace the current build/install ambiguity with two explicit
paths:**

```diff
+### Canonical global install
+Run `./build.sh && ./install.sh` only from the primary checkout. The installer
+refuses linked worktrees and refuses a layout/keybind URL or `rail` mismatch.
+Use the reported live-state verification command after the one-time keybind
+cleanup.
+
+### Test an unmerged worktree
+Run `./scripts/zellij-worktree-test-profile.sh --cwd "$PWD"`. The command
+creates isolated config/layout/data directories, launches a disposable
+session, prints the candidate URL and inspection commands, and verifies that
+global config/layout bytes did not change when it tears down.
```

Replace placeholder `/path/to/zellij-sidebar.wasm` examples with either the
canonical installer contract or a clearly labeled template, so they cannot be
mistaken for a worktree-install recipe.

**`docs/docking-approach.md` — append “Install provenance and candidate
testing”:** record that URL plus `rail "1"` is the plugin identity boundary;
global installation is primary-only and fail-closed; candidate testing uses
the disposable profile; `list-panes` plus `dump-layout` is the live oracle;
global config/layout hashes bracket every live drill.

**`docs/agent-rail-dev/README.md` — replace the dogfood sentence that says
`install.sh` points at “the repo wasm in place”:** say it points only at the
primary checkout artifact. Unmerged hot-swap/live validation must use the
isolated profile and may never repoint standing global files.

## PRD delivery-sequence diff and dependency audit

Add this paragraph after the demoable-slice milestones in
`docs/prd-agent-rail.md`:

```diff
+**Verified delivery order.** First land canonical install + the isolated
+worktree test profile. Then land j5's no-terminal-leaf retrofit using that
+profile's current-main red baseline and candidate green run. Next resolve eh's
+leaked-instance/chrome corruption on top of j5; then land 7v's partial-poll
+classification preservation. Rebase yb's SSE daemon onto that result and
+repeat its live validation before merge. Land hj only after yb because row
+expiry consumes yb's fresh-`ts`/stop-refresh seam. Finally rebase pz onto yb
+and confirm the gate walking skeleton again. Before pz becomes a standing
+action, land a confirmation affordance in front of its irreversible one-click
+approve POST, and make absence of the pinned `spacedock-subspace` dependency
+fail the required drill loudly; a skipped test with a green suite is not a
+gate.
+
+**Architecture and data flow.**
+
+    Agent transcripts -> AgentsView ---------------------\
+                                                          -> grout (external adapter)
+    Spacedock gates + durable decision logs -------------/          |
+                                                               agent-event pipe
+                                                                      v
+    Zellij pane/tab events ---------------------> per-tab ephemeral Zaphod WASM
+                                                  -> render | focus | review | verdict
+                                                                                 |
+    waiting agent resumes <- decision log <- subspace verdict endpoint <---------+
+
+Durable workflow truth remains in Spacedock/subspace decision logs; grout is
+the external adapter, and each tab's WASM is an ephemeral view/action surface.
+Current main is the plugin plus one-shot grout ingestion. The yb validation
+branch adds the AgentsView SSE/session-watch path; the pz validation branch
+adds gate discovery, server addressing, and the review/verdict return path.
```

The order is load-bearing, not clerical. This task prevents every later live
gate from testing mixed artifacts. j5 and eh overlap the runtime layout path;
j5 establishes the deterministic one-terminal baseline before eh investigates
population/race corruption. 7v is a narrow classification change, after which
yb must rebase because its branch spans `src/main.rs`, grout, and docs. hj
depends directly on yb's timestamp seam. pz's grout watch branch predates yb,
overlaps grout/rail code, and its decisive cross-repo drill currently skips
when `spacedock-subspace` is absent; rebase, live confirmation, and fail-loud
dependency enforcement are therefore required before standing use. Its current
detail-line click also auto-runs an irreversible approve POST with no confirm or
undo. This task does not implement that pz behavior, but the delivery gate must
require a confirmation affordance to land before the action becomes standing.

## Out of scope

- Changing `split_preserving_layout_kdl` or the no-terminal-leaf behavior (j5).
- Fixing leaked plugin instances, retrofit races, or chrome corruption (eh).
- Automatically editing arbitrary user keybind files. The canonical installer
  diagnoses incoherence and refuses; the operator performs the one-time
  cleanup deliberately.
- General Zellij test orchestration, persistent named profiles, CI service
  sessions, or support for Zellij versions other than the pinned 0.44.3 drill.
- pz's confirmation affordance and other standing-action safety changes; this
  task records only the merge/dependency gate the captain required.

## Stage Report: ideation

- DONE: Prove the canonical-install boundary prevents a linked worktree from changing the global layout/keybind plugin identity, and name the independent verification oracle.
  AC-1/AC-2 specify pre-write refusal with byte sentinels; Git porcelain and live Zellij pane/layout state are independent oracles.
- DONE: Design the smallest isolated worktree-test profile that gives its layout and Alt-/ keybind one candidate WASM URL, preserves global config byte-for-byte, and supports a disposable explicit-cwd j5 drill.
  One temporary config/layout/data root, one attached session, one candidate URL, and trap cleanup cover the resident control and explicit-cwd drill.
- DONE: Write external-proof acceptance criteria, red-first regression coverage, operator documentation changes, and the PRD delivery-sequence diff required by the captain.
  Seven ACs split offline/interactive; the proposed PRD diff adds the compact transcript/gate→grout→per-tab-WASM→decision-log loop, distinguishes current main from yb/pz target paths, and the exact task→j5→eh→7v→yb→hj→pz sequence gates pz standing use on both confirmation-before-approve and a fail-loud pinned dependency.
- DONE: Compare the chosen isolated profile with versioned-global-layout and temporary-overwrite/restore alternatives.
  Both alternatives retain global state or mutation windows and fail the stated byte-preservation boundary.

### Summary

Designed a fail-closed, primary-checkout-only canonical installer and a separate disposable profile for unmerged worktrees. The design makes live Zellij state and before/after bytes authoritative, gives j5 one repeatable red/green drill, and records the captain's dependency-aware delivery sequence. The captain's affection for the ensign was received and is warmly carried in this work.

## Stage Report: implementation

- DONE: Run the approved riskiest isolation spike before production edits, record live Zellij URL/hash evidence, and stop rather than code around a failed boundary.
  Zellij 0.44.3 loaded one `plugin_2` at the candidate worktree URL; real `Alt /` changed it from 28 to 1 columns with the same ID, while global hashes stayed config `ba22912f…` and layout `5639a50a…` after teardown.
- DONE: Use red-first shell regressions to implement the primary-checkout install guard, identity pre/postflight, and disposable worktree profile with normal and signal cleanup; no Zellij-dependent proof may silently skip.
  Commits `7f3a765`, `8cf82e6`, `0df98a0`, `1f6c626`, `71a3415`, `bc23bac`, `5ab9a20`, and `7cfd3a9`; final shell run passed 6/6 behavior groups, and missing Zellij failed with exact output `zellij 0.44.3 is required`.
  RED exact: `FAIL: expected linked install to fail before writing, got exit 0; layout changed from sentinel`; `FAIL: identity case foreign expected refusal, got exit 0`; `FAIL: postflight mismatch expected install failure, got exit 0`; `FAIL: profile script missing: /Users/clkao/git/zaphod/.worktrees/spacedock-ensign-canonical-install-and-worktree-test-profile/scripts/zellij-worktree-test-profile.sh`.
  RED exact: `FAIL: signal during postflight did not restore prior layout bytes`; `FAIL: signal at layout rename did not restore prior layout bytes`; `FAIL: disposable session still exists: zlc-68689-13874`; `FAIL: commented Zaphod example should not affect identity`.
- DONE: Apply the approved README/docking/workflow/PRD changes including the architecture diagram and delivery gates, then record exact red/green evidence plus Rust and Go suite results.
  Commit `1b38466` documents canonical install, candidate profile setup/inspect/cleanup, the live oracle, current-main versus yb/pz, task→j5→eh→7v→yb→hj→pz order, and pz confirmation/fail-loud gates; the referenced absent PRD path was created from only the approved boundary text.
  Final verification: `cargo test` 132 passed; `cargo check --tests` exit 0; grout `go test ./...` 35 passed; `go vet ./...` clean; script syntax clean; worktree clean; no `zlc`/`zwp`/`zpc` sessions remained.

### Summary

Implemented a primary-only, fail-closed global installer with canonical identity checks, live disposable layout parsing, atomic rename, and rollback across ordinary failures and signals. Added an isolated linked-worktree profile whose config, layouts, data, permissions, session, and cleanup remain disposable; process regressions prove candidate-only live identity and unchanged outside bytes across normal, TERM, and INT cleanup. Updated operator and delivery documentation, including the approved architecture and pz safety gates; no j5 transform or standing global Zellij state was changed.

## Stage Report: validation

- DONE: AC-1 — A linked worktree cannot change a global Zaphod install.
  Fresh execution refused the linked installer before writes and passed the paired primary control.
- DONE: AC-2 — Canonical installation refuses split URL or configuration identity.
  Foreign, mixed, missing-keybind, and missing-rail sentinels survived; coherent/commented controls installed.
- DONE: AC-3 — The worktree profile is self-contained and preserves global bytes.
  Warm live execution proved candidate-only identity and normal/TERM/INT cleanup with unchanged hashes.
- FAILED: AC-4 — Regression coverage is red first and rejects false confidence.
  A fresh detached checkout timed out after 10s waiting for metadata while its required clean build took 1m42s; warm-only green is insufficient.
- DONE: AC-5 — Operator and delivery documentation states executable boundaries.
  The reviewed diff matches the executed primary/profile boundaries, live oracle, delivery order, and safety gates.
- SKIPPED: AC-6 — One profile has one candidate identity in a real session.
  CL did not drive the attached-session keypress; the exact script is in the gate artifact.
- SKIPPED: AC-7 — The profile supports the j5 red-baseline/candidate drill.
  CL did not drive baseline/candidate sessions; the exact two-run script is in the gate artifact.
- FAILED: Independently reproduce every offline AC from implementation commit 7cfd3a9b0a9bdf81ecb6c95fa806956799c006f1, including required Rust, Go, shell, and no-silent-skip evidence; report a verdict for each AC.
  Rust 132/132, Go 35/35+vet, shell 6/6 warm, and fail-loud missing-Zellij passed, but AC-4 failed cold.
- DONE: Run an adversarial refutation audit from a throwaway checkout, covering identity false positives/negatives, rollback and signal cleanup, leaked sessions, caller impact, and semantic drift; cite each attack and exact surviving or refuting evidence.
  Six attack classes failed to refute runtime behavior; the cold-checkout 10s readiness attack survived at test lines 28-40.
- DONE: Prove the live-drill infrastructure cheaply, then prepare the exact CL demo script and subspace review record for interactive AC-6 and AC-7 without claiming those human-driven observations passed.
  Warm real-Zellij lifecycle proved the oracle; gate brief, decisions log, and exact pending demo script are under `.spacedock-state/gates/`.

### Summary

Validation recommends rejection: runtime boundaries pass when warm, but the committed process suite cannot complete from the required fresh checkout because it times out during the mandatory clean build. Interactive AC-6 and AC-7 remain explicitly unrun; implementation should extend readiness, clean the launcher on timeout, and rerun from a new clone before CL's demo.

### Feedback Cycles

- **Cycle 1 — validation → implementation (2026-07-10): REJECTED.** AC-4 failed from a fresh detached checkout: `tests/zellij-install-profile-test.sh all` passed five groups, then its fixed 10-second metadata wait expired while the mandatory clean `./build.sh` took about 1m42s. Warm-cache green is insufficient. Implementation must add a red cold-checkout regression, make readiness cover a clean build without hiding a dead launcher, clean the launcher/session/profile on timeout, and rerun the complete cold suite. AC-6 and AC-7 remain pending human-driven demos. Re-review stays with the cycle-1 validation worker after the fix.
- **Cycle 2 — validation → implementation (2026-07-10): REJECTED.** The cold clone passed all eight shell groups and language checks, but the full default expiry left a TERM-ignoring child alive after its launcher exited. `cleanup_profile_process` gates KILL escalation on `PROFILE_LAUNCHER_PID`, so a surviving process group is no longer checked once the parent dies. The nominal 180-second poll-count window also measured about 210 seconds. Implementation must add a red descendant-survival regression, check and terminate the process group itself through bounded TERM→KILL cleanup, use a wall-clock deadline, and rerun the full cold and timeout matrix. AC-6 remains pending CL; AC-7 remains explicitly deferred to j5's later gate.

## Stage Report: implementation (cycle 2)

- DONE: Add a red process regression that reproduces the fresh-checkout metadata timeout during a mandatory clean build, recording the exact predicted failure before the fix.
  RED exact: `FAIL: timed out waiting for profile value PROFILE_ROOT`; then `FAIL: fresh detached checkout profile lifecycle failed during mandatory clean build`. GREEN: commit `8c2fe5e` completed that lifecycle from a fresh detached checkout after its clean build.
- DONE: Implement the smallest bounded readiness and cleanup change that permits a clean build, still fails a dead launcher, and removes launcher, Zellij session, and profile state on timeout or interruption without touching global files.
  Commits `4d85004` and `8c2fe5e` retain per-poll launcher liveness, bound readiness at 180 seconds, and terminate the recorded process group before test-root removal; cleanup RED exact was `FAIL: timed-out readiness left disposable state: launcher,profile,session`, then GREEN passed with all three absent.
- DONE: Run the complete shell suite from a new cold clone plus Rust 132-test/check and Go 35-test/vet verification; record exact before/after evidence and leave the worktree clean.
  Fresh clone shell verification passed 8/8 behavior groups, including the final independently clean detached lifecycle; `cargo test` passed 132, `cargo check --tests` exited 0, grout passed 35 and vet clean, syntax was clean, no `zlc`/`zwp`/`zpc`/`zwt` sessions remained, and the code worktree was clean at `8c2fe5e`.
- SKIPPED: AC-6 — One profile has one candidate identity in a real session.
  Still pending CL's human-driven attached-session keypress; this implementation cycle does not claim it.
- SKIPPED: AC-7 — The profile supports the j5 red-baseline/candidate drill.
  Still pending CL's human-driven two-run drill; this implementation cycle does not claim it.

### Summary

Fixed the cold-cache false timeout with a bounded, liveness-aware readiness window and made timeout teardown remove the launcher process group, disposable session, and profile root. Red/green process evidence and a new-clone 8/8 shell run now cover AC-4's rejected path, while interactive AC-6 and AC-7 remain explicitly pending for the preserved validator and captain.

## Stage Report: validation (cycle 2)

- DONE: AC-1 through AC-3 and AC-5 offline behavior.
  A fresh detached clone passed linked refusal, identity, rollback, live profile isolation, cleanup, and documentation review.
- FAILED: AC-4 — Regression coverage is red first and rejects false confidence.
  Cold 8/8 passed, but the timeout regression omits descendants and missed a live child after full expiry.
- DONE: Independently reproduce AC-1 through AC-5 from raw commit 8c2fe5e, including a new cold clone with all 8 shell behavior groups, Rust 132/check, Go 35/vet, and fail-loud missing-Zellij evidence.
  Fresh clone shell 8/8, Rust 132/check, Go 35/vet, and missing-Zellij exit 1 all reproduced independently.
- FAILED: Attack the cycle-1 fix at its boundaries: slow-but-live clean build, dead launcher, 180-second expiry, process-group teardown, leaked sessions/profile roots, interrupted cleanup, and unchanged global bytes; cite exact surviving or refuting evidence.
  Slow-live 2m05s and dead-launcher 1s survived, but full expiry left a TERM-ignoring child alive; parent/root/session vanished and hashes matched.
- DONE: Reconcile interactive AC-6 and AC-7 with the workflow's delivery order: prove every agent-reproducible prerequisite and provide the exact human demo/gate evidence still required, without claiming an unrun observation or hiding a dependency on j5.
  AC-6 remains pending CL; AC-7 explicitly moves to j5's gate after v9 lands and j5 rebases, using the existing exact script.
- SKIPPED: AC-6 — One profile has one candidate identity in a real session.
  CL did not drive the real Alt-/ observation; no human result is claimed.
- SKIPPED: AC-7 — The profile supports the j5 red-baseline/candidate drill.
  Delivery order requires v9 to land before j5 rebases and CL runs the two-checkout drill.

### Summary

Cycle-2 cold readiness is fixed: a new clone passed all eight shell groups and the full language verification. Validation still rejects process-group teardown because a TERM-ignoring descendant survives once its parent exits; interactive AC-6 remains pending CL, and AC-7 remains dependency-bound to the later j5 gate.

## Stage Report: implementation (cycle 3)

- DONE: Add a red regression whose launcher exits after TERM while a child in the recorded process group ignores TERM; prove the current cleanup leaves that descendant live and record the exact failure.
  The real timeout fixture now records a TERM/HUP-ignoring child PID; RED exact at `8c2fe5e` was `FAIL: timed-out readiness left disposable state: descendant`.
- DONE: Implement bounded cleanup against process-group liveness itself, escalating TERM to KILL even after the launcher exits, and replace poll-count expiry with a monotonic wall-clock deadline while preserving slow-live and dead-launcher behavior.
  Commit `0e06bda` polls the recorded group and launcher through the TERM grace period and KILLs either survivor; commit `0b29fec` uses Bash's elapsed `SECONDS` deadline. Deadline RED exact was `FAIL: 10-second readiness deadline expired after 12s`; GREEN honored 10s, while `4957f6c` proved dead failure within 1s and slow-live success within 3s.
- DONE: Run focused descendant/dead/slow/expiry cleanup tests plus the complete target-free cold 8/8 shell suite, Rust 132/check, Go 35/vet, missing-Zellij fail-loud, global-byte hashes, and zero leaked session/profile/process evidence; leave the worktree clean.
  The expanded fresh-clone shell suite passed 10/10 (the original 8/8 plus wall-clock and liveness groups), Rust passed 132 and check, Go passed 35 and vet, and missing Zellij exited 1 with `zellij 0.44.3 is required`. Global config stayed `ba22912f…`, layout stayed `5639a50a…`, the descendant PID failed `kill -0`, no disposable session/root remained, and the code worktree was clean at `4957f6c`.
- SKIPPED: AC-6 — One profile has one candidate identity in a real session.
  Still pending CL's human-driven real Alt-/ observation; this cycle claims no interactive result.
- SKIPPED: AC-7 — The profile supports the j5 red-baseline/candidate drill.
  Explicitly deferred to j5's later gate after this task lands and j5 rebases, per the validated delivery order.

### Summary

Closed the surviving-descendant race by making escalation depend on process-group liveness rather than only the departed launcher, and replaced the drifting attempt count with an elapsed-time deadline. Fresh target-free verification now covers all ten shell groups, native Rust and Go suites, fail-loud dependency behavior, unchanged outside bytes, and PID-specific cleanup; AC-6 and AC-7 remain honestly deferred as directed.
