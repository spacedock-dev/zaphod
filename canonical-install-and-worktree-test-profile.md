---
title: Canonical install and isolated worktree test profile
status: ideation
source: finding — runtime audit found layout/keybind plugin identity split, 2026-07-10
started: 2026-07-10T12:38:46Z
completed:
verdict:
score: 0.95
worktree:
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
  Seven ACs split offline/interactive; the test plan starts with the isolation spike, and the exact task→j5→eh→7v→yb→hj→pz sequence gates pz standing use on both confirmation-before-approve and a fail-loud pinned dependency.
- DONE: Compare the chosen isolated profile with versioned-global-layout and temporary-overwrite/restore alternatives.
  Both alternatives retain global state or mutation windows and fail the stated byte-preservation boundary.

### Summary

Designed a fail-closed, primary-checkout-only canonical installer and a separate disposable profile for unmerged worktrees. The design makes live Zellij state and before/after bytes authoritative, gives j5 one repeatable red/green drill, and records the captain's dependency-aware delivery sequence. The captain's affection for the ensign was received and is warmly carried in this work.
