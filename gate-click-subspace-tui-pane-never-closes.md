---
id: s60erz6gam8d9yc238askrz1
title: Gate-click subspace-tui pane never closes — response discarded, CommandPaneExited not subscribed
status: ideation
source: finding — live session dogfooding, 2026-07-08
started: 2026-07-08T08:31:05Z
completed:
verdict:
score: 0.6
worktree:
issue:
pr:
mod-block:
---

## Problem

Clicking a gate row floats `subspace-tui` on the gate's brief
(`handle_click`'s `ClickAction::FloatGate` arm, `src/main.rs:740-758`, via
`open_command_pane_floating`). Live test, 2026-07-08: CL clicked a gate row,
left a comment in `subspace-tui`, pressed `q` to quit — the pane stayed
open and had to be killed manually. This contradicts the
`using-subspace-tui` skill's documented behavior (`q` — "Quit → emit the
feedback fold to stdout"), which assumes the CLI `zellij run
--close-on-exit` flow. That flag is CLI-only and was never available to
zaphod's plugin-API call path in the first place.

Two compounding, confirmed gaps (read against `zellij-tile`/`zellij-utils`
0.44.3 source):

1. **The response is discarded.** `main.rs:750-758` calls
   `open_command_pane_floating(CommandToRun { path: "subspace-tui", args,
   cwd: None }, None, BTreeMap::new())` as a bare statement — the return
   value (a `PaneId`) is never bound to a variable, so the plugin has no
   handle to the pane it just opened.
2. **`CommandPaneExited` is never subscribed.** Zellij's `Event` enum
   (`zellij-utils::data`) defines `CommandPaneExited(u32 terminal_pane_id,
   Option<i32> exit_code, Context)` — fired exactly when a command pane's
   process exits — but zaphod's `subscribe(&[...])` call
   (`src/main.rs:414-421`) lists only `PaneUpdate`, `TabUpdate`, `Mouse`,
   `Key`, `Timer`, `PermissionRequestResult`. There is no code path that
   could close the pane even if (1) were fixed, since the exit event never
   reaches the plugin.
   `CommandToRun` itself (`zellij-utils::data::CommandToRun`) has only
   `path`/`args`/`cwd` — no close-on-exit field exists on the struct the
   plugin API accepts, confirming this can't be fixed by passing a flag;
   it needs the capture-id-then-close-on-exit pattern instead.

## Proposed approach

1. Capture the `PaneId` returned by `open_command_pane_floating` at the
   `ClickAction::FloatGate` call site and track it (a small set/map keyed
   by the gate's `log_path` or just a `Vec`/`BTreeSet<u32>` of
   "gate-review panes we opened," since more than one gate could be
   floated before either closes).
2. Add `EventType::CommandPaneExited` to `subscribe(&[...])`.
3. In `update()`, handle `Event::CommandPaneExited(terminal_pane_id, _exit_code,
   _context)`: if `terminal_pane_id` is in the tracked set, call
   `close_terminal_pane(terminal_pane_id)` and remove it from the set.
4. Confirm `PaneId` from `open_command_pane_floating`'s response carries a
   `Terminal(u32)` variant matching `CommandPaneExited`'s bare `u32` (the
   two APIs use different id shapes — verify the exact conversion during
   implementation, not assumed here).

## Acceptance criteria

**AC-1 — The floated subspace-tui pane closes automatically when the user
quits it (`q`/`ctrl+c`), matching the `using-subspace-tui` skill's
documented behavior.**
Verified by (interactive): click a gate row, quit `subspace-tui` with `q`
— the floating pane closes without manual intervention. Verified by
(offline, to the extent the plugin's pane-tracking logic is a pure
function extractable from host calls): a unit test asserting the tracked
pane-id set drops an id and only that id closes on a matching
`CommandPaneExited`.

**AC-2 — A second gate floated before the first closes is tracked
independently; closing one does not close the other.**
Verified by: a unit test opening two tracked ids and asserting a
`CommandPaneExited` for one does not affect the other's tracked state.

## Test plan

Mostly offline (the tracked-set logic is a pure data structure), with one
interactive spot-check for AC-1 since it depends on zellij's actual
`CommandPaneExited` delivery timing and the `PaneId`-to-`u32` id-shape
question named in Proposed approach step 4 — confirm that once, live,
before trusting the offline tests alone.

## Out of scope

The gate-row click/float mechanism itself (`ClickAction::FloatGate`,
`brief_path_for_log`) — working correctly today, not part of this finding.
Any change to `subspace-tui`'s own `q` behavior — it already does the
right thing (exits cleanly, prints the fold); the gap is entirely on
zaphod's side of not reacting to that exit.
