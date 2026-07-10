---
title: Zellij transactional retained-pane override for safe foreign-tab docking
status: ideation
score: 0.95
source: j5 cycle-5 host-API blocker — captain selected Choice A, 2026-07-10
started: 2026-07-10T15:03:06Z
completed:
worktree:
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

**AC-6 — Zaphod consumes the result and never ships a fork.** Zaphod arms swap
steering only for `Applied`; `Rejected` clears the pending request and leaves
the tab alone. The repository contains no vendored Zellij source, `[patch]`,
fork URL, or install step that replaces the user's Zellij binary.

Verified by: a red-first Zaphod integration test that feeds matching Applied
and Rejected events through the plugin state machine and observes emitted host
actions, plus a clean-package/install review against the implementation diff.
The host test runs against an explicitly supplied disposable upstream patch
artifact.

### Interactive

**AC-I1 — CL sees the N=2 value outcome.** In a disposable two-shell stacked
tab, one `Alt /` press produces the same two terminal IDs and processes beside
one rail; a second press changes only dock state.

Verified by: CL's live demo paired with before/after `list-panes --json` and
PID output. Visual inspection alone does not settle it.

**AC-I2 — A focus switch cannot dock the wrong tab.** CL starts a retrofit on
tab T, immediately changes tabs, and sees either an applied rail on T or an
unchanged T with a visible rejection. The newly focused tab remains byte-for-
byte equal in its pane listing.

Verified by: CL's live two-tab demo with pre/post pane listings and the result
event's stable tab ID.

## Test plan

1. **Run the preserved red case first.** After disk pressure clears, run only
   `transactional_override_rejects_unsatisfiable_n2_without_losing_terminals`
   against unpatched `v0.44.3`. Record the expected missing terminal or missing
   API failure, not a compile error. Keep the existing installed-binary planner
   spike as supporting evidence: `{5,6}` fitted; `{7,8}` rejected unchanged.
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
7. Only after the upstream patch passes steps 1-6 may Zaphod add its result
   handler and run AC-6. Finish with AC-I1 and AC-I2 in a disposable session.

The design extends Zellij's existing pure
`TiledPaneLayout::position_panes_in_space` and `insert_children_nodes`
functions. It replaces no Zaphod pure function until the host contract passes.

## Proposed doc diff

Until AC-1 through AC-6 pass, keep the current warning and add this blocker to
`SPEC.md` landmine #26: retained override in 0.44.3 neither expands `children`
to the retained count nor rolls back failed insertion. After the host contract
passes, revise `docs/docking-approach.md` and SPEC to say:

> Foreign-tab retrofit uses a stable-tab-ID transactional override. The host
> plans every retained pane into non-spawning `children` positions before it
> stages the rail. Rejection leaves the tab unchanged; success is explicit and
> swap steering begins only after the matching result event.

README should promise exact first-toggle terminal preservation only after the
offline matrix and CL demo pass. No document should instruct users to install
a Zellij fork.

## Out of scope

- Changing legacy `override_layout` semantics or making multi-tab override
  transactional in v1.
- Transactional floating-layout replacement or spawnable terminal leaves.
- Reading current cwd, current command, or serialized resurrection KDL as
  original typed launch identity.
- Post-hoc cleanup, rollback after process loss, or guessing which terminal a
  new pane replaced.
- eh's floating-instance leak, actor election, and chrome-placement work.
- Shipping, vendoring, pinning, or requiring a Zellij fork. The disposable
  patch exists only to prove an upstream contract. Any shipped fork requires a
  later, explicit captain gate; this entity does not authorize one.

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
