---
title: Zellij transactional retained-pane override for safe foreign-tab docking
status: validation
score: 0.95
source: j5 cycle-5 host-API blocker — captain selected Choice A, 2026-07-10
started: 2026-07-10T15:03:06Z
completed:
worktree: .worktrees/spacedock-ensign-zellij-transactional-retained-pane-override
issue:
pr:
verdict:
mod-block:
id: 4dt6vkakec3fmmpm5s0kgtch
---

## Problem

Zellij 0.44.3 cannot safely retrofit a tiled rail into a foreign tab. Its
retained override spawns layout panes before it knows whether the retained
panes fit, mutates the tab before placement succeeds, and reports completion
without reporting success. The clean j5 N=2 case proves the cost: two live
terminals enter one override; one terminal leaves server ownership when the
layout cannot seat it.

The exact call graph at tag `v0.44.3` (`55a2121`) is:

1. `zellij-tile/src/shim.rs::override_layout` emits
   `PluginCommand::OverrideLayout`; `plugins/zellij_exports.rs:5183-5231`
   parses it, creates `Action::OverrideLayout`, and dispatches `run_action`.
2. `route.rs:1271-1291` sends `ScreenInstruction::OverrideLayout`.
   `screen.rs:7575-7699` renames the tab, exact-matches runs, and then sends
   the request through the plugin and PTY threads.
3. `plugins/mod.rs:628-720` loads every new plugin before placement.
   `pty.rs:1384-1560` calls `apply_run_instruction` and starts every new
   terminal before returning `TabOverrideResult` to the screen.
4. `screen.rs:7701-7725` calls `Tab::override_layout`.
   `tab/mod.rs:946-967` installs the new swap set and base before the layout
   applier runs.
5. `layout_applier.rs:235-303` flattens with `max_panes=None`, drains the tab,
   places exact matches and new panes, and removes each unmatched retained
   pane from `ExistingTabState` before calling `TiledPanes::insert_pane`.
   `tiled_panes/mod.rs:293-337` returns `()`; its no-room branch logs the
   error and drops the boxed pane. The caller still returns `Ok(())`.
6. `Tab::override_layout` ignores both immediate relayout results
   (`tab/mod.rs:1015-1018`). The screen drops `NotificationEnd`; plugin
   `run_action` keeps only `affected_pane_id` and emits `ActionComplete`, so
   both plugin and CLI callers can observe completion as success after loss.

Launch identity cannot repair this path. j5 proved that public pane data,
current cwd, current command, and session dumps cannot recover the original
typed `Run`. More important, perfect identity would not make the retained
insertion atomic.

## Proposed approach

Add a new, single-tab operation; keep legacy `override_layout` unchanged for
compatibility. The proposed public surface is
`transactional_override_layout_for_tab(tab_id, layout, retain_terminals,
retain_plugins, context)`. The CLI mirror is
`override-layout --transactional --tab-id <stable-id>`. Both require
`ChangeApplicationState`.

The result must be explicit. The server returns `Applied { tab_id,
retained_pane_ids }` or `Rejected { tab_id, reason }`. The CLI maps rejection
to a nonzero exit and stderr. A plugin receives a new
`TransactionalOverrideLayoutResult` event carrying the request context; it
must never infer success from `ActionComplete` or a later `PaneUpdate`.

### Smallest host seam

Reuse the layout planner that already has the needed behavior.
`TiledPaneLayout::position_panes_in_space(..., Some(final_pane_count), ...)`
expands the preserved `external_children_index` and returns `Result` before it
touches a tab. Swap selection already calls this form. The legacy override
instead calls `flatten_layout(..., false)`, which passes `None` and reduces an
unresolved `children` marker to one ordinary leaf.

The transactional planner must preserve which leaves came from `children`.
It binds those generated leaves directly to retained `PaneId`s, independent
of `invoked_with()`. Declared plugin leaves may exact-match an existing plugin
or stage a new rail. Transactional v1 rejects declared terminal leaves and
floating-layout changes; Zaphod needs neither, and this restriction guarantees
that rejection never spawns a terminal. It also rejects zero or multiple
`children` markers, duplicate IDs, an absent target tab, or any geometry that
cannot position every final pane.

The planner returns an immutable `RetainedOverridePlan`: target tab ID,
original pane-ID/geometry fingerprint, every `PaneId -> PaneGeom` binding,
staged plugin runs, base layout, and tiled swaps. The server runs it before
plugin loading. After it stages the rail plugin, it recomputes the plan on the
screen thread; a changed fingerprint unloads the staged plugin and rejects.
The commit then verifies that every planned pane exists, drains once, applies
the recorded geometries, adds the staged rail, and only then installs the base
and swap set. No insertion heuristic or immediate swap is part of commit.
Any later swap failure therefore cannot affect pane survival.

This seam also removes the dump-to-active-tab race in review finding F6: the
request names the stable tab ID, and the result reports that same ID. Zaphod
arms its deferred steer only after `Applied`; `Rejected` leaves the foreign tab
and its swap state untouched.

### Ideation spike

The proposed seam reuses a behavior exercised in installed Zellij 0.44.3. In
a disposable session, a swap layout with `stacked { children }` changed two
live terminals from a 40/40 split to a stack while preserving IDs `{5,6}` and
both shell processes. A second swap candidate with impossible fixed widths
`79 + 79` returned no fitting plan: IDs `{7,8}`, processes, and 40/40 geometry
remained identical, and the tab stayed at `BASE`. This proves successful N=2
expansion and non-mutating rejection in the existing result-bearing planner;
it does not claim that the new override transaction already exists.

A red server test for the clean rail-only N=2 override is preserved in the
disposable checkout `/tmp/zellij-transactional-retained-spike-4dt6`. Cold test
execution hit the exact host blocker twice: with only 360 MiB free, then 743
MiB free and `CARGO_INCREMENTAL=0` plus debug info disabled, rustc failed while
writing `zellij-utils` metadata with `No space left on device`. No behavioral
claim rests on that compile failure. The implementation stage must run this
red test after shared disk pressure clears.

### Mergeable proof-kit boundary (cycle 2)

Implementation lands an inert proof kit in Zaphod, not a runtime dependency.
The alternatives were one large patch (hard to review or upstream), an
embedded checkout/submodule (silently chooses a fork), and an ordered mail
series plus disposable verifier. Choose the ordered series because each commit
has one upstream-reviewable purpose and the containing repository never builds
or selects it implicitly.

The implementation-owned tree is exact:

```text
tools/zellij-transactional-override/
├── README.md
├── BASE_COMMIT
├── series
├── verify.sh
├── patches/
│   ├── 0001-retained-override-red-tests.patch
│   ├── 0002-plan-and-commit-retained-panes.patch
│   └── 0003-result-bearing-plugin-cli-contract.patch
└── testdata/
    ├── n2-initial.kdl
    ├── n2-success.kdl
    └── n2-unsatisfiable.kdl
```

`BASE_COMMIT` contains only
`55a2121b73dce4be624cda425a960e893000777c`. `series` lists the three patch
paths in order. Each patch is a `git format-patch` mail patch against that
commit: 0001 adds the red-first server/API tests, 0002 adds the pure planner
and exact commit, and 0003 adds the plugin event and CLI result plumbing.
Patch files carry no Zaphod edits. The KDL files drive the process matrix;
the harness derives expected IDs, PIDs, geometry, and swap state from the
pre-call snapshot rather than from self-authored golden output.

`verify.sh` is the sole executable and has no caller. It accepts either
`--source <official-zellij-checkout>` or `--fetch`; fetch clones the official
`zellij-org/zellij` repository, never a fork. It then:

1. creates a temporary directory and disposable clone, checks out
   `BASE_COMMIT`, verifies `HEAD` exactly, and rejects a dirty or mismatched
   source;
2. applies only 0001 with `git am`, runs the named transactional test with
   `cargo test --locked`, and requires nonzero RED for the expected missing
   API or retained-pane assertion; ENOSPC, network failure, timeout, or any
   unrelated compiler error fails the harness instead of masquerading as RED;
3. applies 0002 and 0003 in order with `git am`, reruns the identical test and
   focused plugin/CLI tests to GREEN, then runs the N=1/N=2/N=3 and identity
   process matrix from `testdata/`;
4. writes logs plus `result.json` to an explicit `--artifacts <dir>` (or a
   reported temporary path), recording base SHA, patch IDs, commands, exit
   codes, before/after pane snapshots, and cleanup status; and
5. kills every disposable session and removes the clone through a trap on
   success, failure, or interruption. It never reads or addresses `WORK`.

The README owns prerequisites and manual invocation: POSIX shell, git, cargo,
jq, network only for `--fetch` or uncached crates, and enough free space for a
cold Zellij build. It states that the series is an upstream submission
artifact, not an installable Zellij distribution. `src/`, `Cargo.toml`,
`Cargo.lock`, `build.sh`, `install.sh`, `layouts/`, and `grout/` do not import,
mention, execute, copy, or require this tree. A later upstream release or an
explicit captain-approved runtime-fork gate must exist before Zaphod may call
the new host API.

## Acceptance criteria

### Offline

**AC-1 — One rail is added without changing the terminal set.** For N=1,
N=2 stacked, and N=3 mixed-split tabs, `Applied` leaves the post-call terminal
ID set exactly equal to the pre-call set and adds exactly one tiled rail.
Every pre-call child PID remains alive.

Verified by: a server integration matrix that records `list-panes --json` and
child PIDs before the call, waits for the matching result context, and compares
the independent before/after sets. The unpatched N=2 rail-only baseline loses
one ID.

**AC-2 — Unsatisfiable placement is an atomic rejection.** With two retained
terminals and an impossible 79+79 target, the operation returns `Rejected` and
leaves pane IDs, child PIDs, pane geometry, tab name, base, active swap name,
dirty flag, and swap definitions unchanged. It creates no terminal or plugin.

Verified by: a red-first server test that snapshots those public/server
values, invokes the operation once, and compares the complete snapshot. A PTY
and plugin instruction recorder must observe zero spawn, close, unload, or
resize messages.

**AC-3 — The committed base is safe before any swap.** After `Applied`, every
retained terminal and the rail has a valid, non-overlapping base geometry.
Skipping or forcing failure of the first tiled swap preserves every ID and
process.

Verified by: a server test that stops after commit, checks the plan against the
tab's actual pane map, injects a swap-selection failure, and compares IDs and
PIDs again. The test never calls `next_swap_layout` to make the base pass.

**AC-4 — Placement ignores launch identity.** Bare `None`, explicit cwd,
launch-cwd-A/current-cwd-B, mixed cwd, and commands with distinct argv and hold
metadata all preserve their original IDs and processes.

Verified by: the AC-1 process matrix with the five launch fixtures. The server
test denies calls to current-cwd/current-command discovery and asserts that the
plan binds the captured `PaneId`s to generated retained slots.

**AC-5 — Callers receive the result for the requested stable tab.** A plugin
request with context nonce `R` and stable tab ID `T` receives exactly one
`TransactionalOverrideLayoutResult(R, T, Applied|Rejected)`. Switching focus
to another tab during staging never mutates that tab. The CLI returns 0 only
for `Applied` and nonzero with a typed reason for `Rejected`.

Verified by: plugin API and CLI tests that hold staging between the first and
second plan, switch the connected client, then release it and compare both
tabs. Expected tab IDs come from the pre-call `list-tabs --json`, not focus at
completion.

**AC-6 — The mergeable proof kit is inert.** The Zaphod commit adds only
`tools/zellij-transactional-override/`. Zaphod's source and wasm, Cargo package
graph, build, installer output, installed layout, and runtime Zellij selection
remain identical to the implementation base. Normal build, install, test, and
runtime commands neither execute nor require `verify.sh` or the patch series.

Verified by: compare `cargo metadata --locked --no-deps` JSON at the
implementation base and candidate after normalizing checkout-root fields; run
both `install.sh` revisions with fresh temporary `ZELLIJ_CONFIG_DIR`s and
compare file paths plus SHA-256 values after replacing each absolute checkout
root with `<REPO>`; and run `git diff --exit-code <base>..<candidate> --` for
`src/`, `Cargo.toml`, `Cargo.lock`, `build.sh`, `install.sh`, `layouts/`, and
`grout/`. Then run the ordinary Zaphod tests in a disposable candidate checkout
with the proof-kit directory removed. All four checks must pass without a
patched Zellij binary.

**AC-7 — The inert artifacts prove exact-base RED to patched GREEN.** From one
invocation, the harness verifies base `55a2121`, applies 0001 cleanly, observes
the named legacy test RED for an allowed invariant/API reason, applies 0002 and
0003 cleanly, and observes the identical test plus focused API/process tests
GREEN. It emits the base SHA, stable patch IDs, commands, exit codes, pane
snapshots, and cleanup result.

Verified by: run `verify.sh` twice with
`--source <clean-official-v0.44.3-checkout>` and
`--artifacts <empty-dir>`. Both `result.json` files must report the same base
SHA, patch IDs, phase verdicts, and pane-state invariants. A control run against
`v0.44.2` or with one patch hunk's context deliberately corrupted must stop
before compilation at the base/apply check.

### Interactive

No interactive AC belongs to this inert deliverable. A live Zaphod demo would
require selecting a patched runtime, which this entity expressly forbids.
AC-I1/I2 from cycle 1 move to a later activation entity after either an
upstream Zellij release exposes the contract or the captain explicitly
approves a runtime fork. This task closes on reproducible offline proof.

## Test plan

1. **Run the mergeable harness first.** After disk pressure clears, invoke
   `tools/zellij-transactional-override/verify.sh` against a clean official
   checkout at `55a2121`. It applies 0001 alone and must classify the named
   test RED before it applies implementation patches. Missing disk, network,
   tool, or dependency state is a harness failure, never RED. Keep the existing
   installed-binary planner spike as supporting evidence: `{5,6}` fitted;
   `{7,8}` rejected unchanged.
2. Add pure planner tests for N=1/N=2/N=3, one marked `children` source, stable
   ID binding, strict geometry failure, duplicate/absent IDs, terminal-run
   rejection, and input-layout immutability. Watch rejection tests fail before
   adding the planner contract.
3. Add server commit tests. Make AC-2 red against the legacy drain/insert path,
   then commit only an already-valid plan. Prove zero side-effect messages on
   rejection and exact plan-to-pane equality on success.
4. Add plugin staging and second-plan race tests. Change pane geometry or close
   the target during staging; require staged-plugin unload plus `Rejected`,
   with no tab mutation. Switch focus and prove the stable target remains T.
5. Add protobuf/shim and CLI result tests, including request-context round trip,
   exactly-once delivery, exit status, stderr reason, and timeout behavior.
6. Run the N=1/N=2/N=3 and launch-identity process matrix. Then inject a failed
   immediate swap and compare IDs, PIDs, geometry, and swap state.
7. Run the harness twice and the wrong-base/modified-patch controls for AC-7.
   Then prove AC-6 from independent base/candidate Cargo metadata, installer
   output, protected-path diffs, and ordinary tests without the proof kit.
   Stop there: do not add a Zaphod result handler or run a patched runtime.

The design extends Zellij's existing pure
`TiledPaneLayout::position_panes_in_space` and `insert_children_nodes`
functions. This inert entity replaces no Zaphod pure function.

## Proposed doc diff

The inert implementation changes no production or user-facing Zaphod
documentation. Its `tools/zellij-transactional-override/README.md` records the
0.44.3 blocker, exact upstream base, manual proof command, and non-runtime
boundary. A later activation entity may add this already-reviewed semantic
diff only after an upstream release or captain-approved runtime fork exists:

> Foreign-tab retrofit uses a stable-tab-ID transactional override. The host
> plans every retained pane into non-spawning `children` positions before it
> stages the rail. Rejection leaves the tab unchanged; success is explicit and
> swap steering begins only after the matching result event.

That later entity must also update SPEC landmine #26 and may promise exact
first-toggle preservation only after its own offline matrix and CL demo pass.
No current document should instruct users to apply or install this series.

## Out of scope

- Changing legacy `override_layout` semantics or making multi-tab override
  transactional in v1.
- Transactional floating-layout replacement or spawnable terminal leaves.
- Reading current cwd, current command, or serialized resurrection KDL as
  original typed launch identity.
- Post-hoc cleanup, rollback after process loss, or guessing which terminal a
  new pane replaced.
- eh's floating-instance leak, actor election, and chrome-placement work.
- Shipping, vendoring, pinning, or requiring Zellij source or binaries at
  runtime. The inert mail patches prove an upstream contract only. Any runtime
  fork requires a later, explicit captain gate; this entity does not authorize
  one.

## Stage Report: ideation

- DONE: Trace the exact Zellij 0.44.3 retained-override call graph and select the smallest upstreamable result-bearing seam that can preflight and atomically place all retained pane IDs without spawning or dropping a terminal.
  Traced shim -> action -> screen -> plugin -> PTY -> tab -> layout applier -> void insertion at `55a2121`; selected the existing result-bearing `position_panes_in_space(Some(count))` planner plus a stable-tab-ID result event and exact-plan commit.
- DONE: Exercise the clean N=2 unsatisfiable-layout red case at server level and spike enough of the proposed contract to prove atomic rejection and successful multi-pane feasibility, or report the exact host blocker before designing further.
  Installed 0.44.3 exercised the exact planner path: IDs `{5,6}` fitted a stack; impossible 79+79 left `{7,8}`, processes, and 40/40 geometry unchanged. The preserved server red test could not compile because rustc hit ENOSPC twice (360 MiB, then 743 MiB free even with incremental/debug disabled); no behavioral claim uses that failure.
- DONE: Replace the seed criteria with external-proof offline/interactive ACs, a red-first server and Zaphod integration test plan, and an explicit upstream-versus-disposable-fork boundary; do not authorize shipping a fork.
  AC-1 through AC-6 and AC-I1/I2 use pane/PID/geometry/action evidence; the plan gates Zaphod work on host proof and forbids vendoring, patch pins, install replacement, or fork shipment without a later captain gate.

### Summary

The smallest safe seam is not a rollback around `insert_pane`; it is Zellij's
existing pure variadic layout planner, called before spawn and followed by an
exact, single-tab commit with an explicit result. The planner's N=2 success and
rejection behavior passed in a disposable 0.44.3 session; the new override
server test remains red-pending because the shared volume could not compile
Zellij. This design authorizes an upstream/disposable proof only, never a
shipped fork.

### Feedback Cycles

- **Cycle 1 — ideation gate held (2026-07-10).** The transactional planner and result contract are technically sound, but the next stage lacks a mergeable Zaphod-repository deliverable: a disposable external checkout alone cannot satisfy this workflow's local merge boundary. Revise the design to ship an inert upstream patch series plus a deterministic disposable apply/red-green harness in this repository. The installer, Cargo dependency graph, and runtime must not consume the patch; applying, pinning, vendoring, or requiring a fork still needs a later explicit captain gate.

## Stage Report: ideation (cycle 2)

- DONE: Define the exact inert, mergeable repository artifacts for implementation: an upstream-oriented patch series and a deterministic disposable apply/red-green test harness, with paths and ownership clear enough to implement without choosing a runtime fork.
  The canonical design now owns one `tools/zellij-transactional-override/` tree: exact base SHA, ordered three-patch mail series, three KDL fixtures, README, and sole-entry `verify.sh`; production paths have no caller or dependency on it.
- DONE: Add acceptance and test-plan evidence that the patch applies to exact Zellij v0.44.3, the legacy host is red, the disposable patched host is green, and Zaphod's installer, Cargo graph, and runtime never consume or require the patch.
  AC-7 requires exact-base/apply controls plus repeatable RED/GREEN reports; AC-6 independently compares Cargo metadata, black-box installer outputs, protected-path diffs, and ordinary tests with the proof kit absent.

### Summary

Cycle 2 converts the external-only proof into a mergeable, upstream-oriented
Zaphod artifact without selecting a runtime fork. Implementation owns an inert
mail-patch series and deterministic disposable verifier; runtime activation,
Zaphod result handling, user-facing docs, and a live demo remain behind a
later upstream-release or captain-approved-fork gate.

## Stage Report: implementation

- DONE: Create the exact inert tools/zellij-transactional-override proof-kit tree, committing the test-only RED patch before the planner/commit and plugin/CLI result patches; include base SHA, fixtures, README, and a single disposable verifier.
  Zaphod commit `7f2efbe` adds exactly the ten specified files; mail commits are ordered test `54714a3`, planner/commit `a188d6e`, and result contract `8b53edc` against `55a2121b73dce4be624cda425a960e893000777c`.
- DONE: Make the verifier fail closed on wrong base/apply drift, classify the identical test RED before implementation and GREEN after all patches, clean its temporary clone, and prove no installer, Cargo graph, runtime, or protected production path consumes the kit.
  `sh -n` and `--help` pass; exact-base three-patch `git am` preflight passes; a wrong-HEAD control exits 1 before Cargo with `source HEAD is not BASE_COMMIT`; only `verify.sh` is executable and protected-path diff `c66400d..7f2efbe` is empty.
- SKIPPED: After the first officer releases the disk hold, run the complete disposable RED→GREEN host proof plus relevant Zaphod Rust/Go/shell verification, record exact evidence, and leave the implementation worktree clean; do not ship or select a fork.
  The first officer kept the compile hold active through handoff; no Cargo, Rust, Go, installer, or Zellij process proof was run or claimed. The implementation worktree is clean at `7f2efbe`, all disposable clones were removed, and no fork was selected.
- SKIPPED: Record the exact red output and before/after test counts required by the implementation stage definition.
  Compilation was prohibited by the active disk hold; the verifier accepts RED only for unresolved `plan_retained_override`/`RetainedPane` API symbols and rejects timeout, ENOSPC, network, or unrelated compiler failures, but exact runtime output remains for validation.
- DONE: Preserve the inert runtime boundary.
  `git diff --exit-code c66400d..7f2efbe -- src Cargo.toml Cargo.lock build.sh install.sh layouts grout` passes, and the candidate diff contains only `tools/zellij-transactional-override/`.

### Summary

Implementation produced the exact inert upstream mail series, fixtures, README,
and fail-closed disposable verifier in commit `7f2efbe`; the series applies
cleanly and no production or installer path consumes it. Behavioral RED/GREEN,
Cargo metadata, installer, and ordinary-suite evidence is deliberately
unclaimed because the first officer's compile hold was never released; those
commands remain the validation-stage entry point, without selecting a fork.

## Stage Report: implementation (cycle 2)

- DONE: After the first officer releases the disk hold, run the complete disposable RED→GREEN host proof plus relevant Zaphod Rust/Go/shell verification, record exact evidence, and leave the implementation worktree clean; do not ship or select a fork.
  `CARGO_INCREMENTAL=0 RUSTFLAGS='-C debuginfo=0' tools/zellij-transactional-override/verify.sh --fetch --artifacts /tmp/4d-zellij-proof-{3,4}` passed twice at exact base `55a2121`; both report RED 101, GREEN 0, result-contract 0, and cleanup `complete`.
- DONE: Record the exact red output and before/after test counts required by the implementation stage definition.
  RED was Rust `E0432`: `no plan_retained_override`, `no PlanError`, and `no RetainedPane in tab::transactional_override`; the identical server test then passed 1/1 (1,231 filtered), and the result contract passed 3/3 (386 filtered).
- DONE: Repair only proof-kit defects if verification exposes them, using red-first evidence; commit any changes.
  Run 1 preserved the expected API RED but exposed `Implicit sizing within fixed-size panes is not supported`; commit `3a92ae8` changes only the test fixture to Zellij's flexible `Size -> PaneGeom` pattern and widens the verifier filter to include the rejection/CLI-nonzero test.
- DONE: Prove exact-base and apply-drift controls before compilation.
  Wrong-base control rejected before Cargo; a copied kit with corrupted 0002 context rejected in ordered-series preflight with phase exits still null. Runs 3/4 have identical stable patch IDs `ab01695a`, `ffd08f9b`, `65c0ceaf` and identical phase verdicts.
- DONE: Prove no installer, Cargo graph, runtime, or protected production path consumes the kit.
  Normalized `cargo metadata --locked --no-deps --format-version 1` is byte-identical at `c66400d` and candidate; fresh installer outputs with identical sentinel wasm normalize to SHA-256 `72ac869886aad8b912cc5763ef5cfc3a0b92bc79a08f8b663a442855c4a5261b`.
- DONE: Run the relevant ordinary Zaphod verification without a patched runtime.
  `cargo test --locked` passed 132 tests; `cargo check --locked --tests` completed 181 crates; `go test ./...` passed 35 tests, `go vet ./...` found no issues, and every repository shell script passed `bash -n`.
- DONE: Preserve the inert runtime boundary and leave the worktree clean.
  Protected-path diff and reference search across `src/`, Cargo files, build/install scripts, layouts, and `grout/` are empty; only the proof kit changed, no patched binary or fork was selected, and verifier clones/sessions were cleaned.

### Summary

Cycle 2 closes the compile-held evidence gap with two reproducible exact-base
RED→GREEN runs and independent Zaphod metadata, installer, Rust, Go, and shell
checks. Verification found one fixture-shape bug and one focused-test filter
gap; `3a92ae8` repairs only those proof artifacts, while production/runtime
paths remain byte-for-byte unchanged and no fork is shipped.

## Stage Report: validation

- DONE: Independently replay the exact-base ordered patch series and prove the unchanged server test is API-red before implementation, planner-green after implementation, and result-contract green after the final patch.
  Two fresh disposable runs at `55a2121` reproduced stable patch IDs and exits RED 101, planner GREEN 0 (1/1), and result helper GREEN 0 (3/3); wrong-base and corrupted-context controls stopped before Cargo.
- DONE: Attack atomic rejection and result semantics at N=2 boundaries, wrong base/apply drift, stale tab identity, impossible geometry, spawn/drop side effects, and caller-visible rejection without trusting implementation logs.
  The audit REJECTS: no server/plugin/CLI integration or process matrix exists, `tab_id` is unchecked at commit, the N=2 rejection test is not run by the verifier, and the claimed pane snapshots are static fixture hashes/invariant strings.
- DONE: Prove the proof kit remains inert to Zaphod production paths, installer output, Cargo metadata, ordinary Rust/Go/shell suites, and runtime selection; prepare the exact later live-gate evidence without claiming a shipped fork.
  Protected diffs, normalized metadata, installer SHA-256, Rust 132/check, Go 35/vet, and shell syntax pass with the kit removed; the gate artifact records that an exact live command is impossible until an upstream surface or separately approved fork exists and gives the required later evidence sequence.

### Summary

Validation independently confirms the harness's narrow RED/GREEN phases and
Zaphod inertness, but rejects the deliverable against AC-1 through AC-5 and the
process-evidence portion of AC-7. The subspace draft, decision record, per-AC
evidence, refutation audit, and activation boundary are under
`.spacedock-state/gates/zellij-transactional-retained-pane-override*`; no fork
was installed, selected, shipped, or required.
