---
id: n5w8b1s9n8zqczvw2ebd25q8
title: Click-target line mapping is unverified at sliver width after the compact renderer
status: ideation
source: finding — named as out of scope by sliver-rendering-lacks-narrow-width-handling's ideation, 2026-07-09
started: 2026-07-09T13:01:33Z
completed:
verdict:
score: 0.5
worktree:
issue:
pr:
mod-block:
---

## Problem

`sliver-rendering-lacks-narrow-width-handling` (now merged) added a compact
1-glyph-per-row renderer (`sliver_lines`) for the undocked 1-col sliver,
explicitly leaving one question unresolved (its own Out of scope section):
click dispatch (`SectionLayout`/`target_for_line`, `decide_rail_click`) was
designed around the docked-width layout's line-counting assumptions (a
header line per section, ~2 lines per row/session/gate). The compact
sliver renderer has a different shape entirely — no header lines, exactly 1
line per item. Nobody has verified whether clicking still targets the
correct row in the sliver now, or whether it silently misfires (wrong row,
or the header-line click that normally toggles the dock).

## Proposed approach

### Trace: (a)/(b) — does the existing click math still work?

`handle_click` (`main.rs:733`) never consults `self.last_cols`. Every
mouse click, at any render width, dispatches through
`decide_rail_click` → `section_layout(pane_count, session_count,
gate_count).target(line)` → (pane region) `target_for_line(line,
pane_count)`. That math assumes the docked shape render() actually prints
in the `cols >= STATUS_MIN_COLS` branch: one header line, then 2 lines
per row, then (if present) a 1-line AGENTS header + 2 lines per session,
then a 1-line GATES header + 2 lines per gate. `sliver_lines` prints a
completely different shape in the early-return branch: no header line at
all, exactly 1 line per row/session/gate, rows then sessions then gates
concatenated with no section boundaries. Nobody updated the click side
when the print side changed — **(a) is false, it does not work by
coincidence**, and **(b) it is silently broken**, in three compounding
ways, worst first:

1. **The first sliver line is swallowed as a phantom header click.**
   `target_for_line` treats line 1 as `LineTarget::Header` unconditionally
   (`main.rs:1897-1899`) because line 1 *is* the header in docked mode.
   The sliver has no header line — line 1 is the first row's own glyph —
   so clicking it always fires `ClickAction::ToggleDock` instead of
   acting on that row.
2. **Every other pane-region line collapses two sliver rows onto one
   docked row index, and the collapse compounds down the list.** Docked
   math allots 2 lines per row; sliver allots 1. For sliver line `L`
   (`L >= 2`), `target_for_line` computes `idx = (L-2)/2` (integer
   division) — so sliver lines 2 and 3 both resolve to row 0, lines 4 and
   5 both resolve to row 1, and so on. Concretely, a 5-pane, no
   session/gate sliver (`sliver_lines` output = 5 lines, one glyph per
   row, verified against `decide_rail_click`/`section_layout` as shipped
   at commit `5194c5d`):

   | Sliver line (visually = row) | Click resolves to | Correct action |
   |---|---|---|
   | 1 (row 0) | `ToggleDock` | `FocusPane(row 0)` |
   | 2 (row 1) | `FocusPane(row 0)` | `FocusPane(row 1)` |
   | 3 (row 2) | `FocusPane(row 0)` | `FocusPane(row 2)` |
   | 4 (row 3) | `FocusPane(row 1)` | `FocusPane(row 3)` |
   | 5 (row 4) | `FocusPane(row 1)` | `FocusPane(row 4)` |

   Every one of the 5 clicks is wrong. Rows 2, 3, and 4 — 60% of the
   list — are permanently unreachable by any click; rows 0 and 1 are only
   reachable by clicking a *different* row's glyph. This is not
   "unclickable," it is silent misrouting: the pane that gets focus is
   never the one the user clicked.
3. **Session/gate glyphs collide with the pane region entirely.** The
   docked math only reaches `agents_header`/`gates_header` past line
   `2 + 2*pane_count`. A sliver with even 1-2 rows plus a session/gate
   never has that many lines total (e.g. P=2,S=1,G=1 → 4 sliver lines,
   but `agents_header` sits at docked line 6) — so every sliver line for
   that config resolves through the pane-only branch, meaning a click on
   the session or gate glyph is misrouted as if it were a pane row (or
   the phantom header), never reaching `SessionRow`/`GateRow` at all.

### Decide: (c) — what should a sliver click do?

Not "fix the offsets so each glyph directly dispatches its row's
action." Two things independently support **dock-then-act** — clicking
anywhere on the sliver expands it to the docked rail (the same action as
the header's `⇄` click today), and never directly fires `FocusPane` /
`FloatGate` from the sliver itself:

- **The glyphs the user would be aiming at are not distinguishable.**
  `sliver_lines` (`main.rs:1967-1975`) renders exactly `state_glyph(...)`
  or `row_marker(...)` per item — a bare colored dot, nothing else. Two
  idle rows, or two blocked gates, render byte-identical lines. Even a
  *correctly* offset direct-dispatch model would be unverifiable by the
  user at the point of the click: there is no title to confirm "yes,
  that's the one," only a line-counting bet in a 1-column terminal pane.
  A `FloatGate` misclick is the sharper case — it floats a whole review
  TUI on the wrong entity's brief, a bigger blast radius than a wrong
  `FocusPane` (instantly visible and one click to undo).
- **The sliver's own design intent is glancing, not acting.** The sibling
  task's ideation (`sliver-rendering-lacks-narrow-width-handling`, now
  archived) frames the compact mode as "the closest thing to the same
  information, minimally" — an at-a-glance indicator, not an alternate
  interactive list. `docs/docking-approach.md`'s adopted architecture
  gives the sliver exactly one prior interaction anywhere in its design
  history: the header's dock-toggle. Generalizing that single existing
  behavior to the whole sliver (rather than inventing new direct-action
  wiring for a surface never designed to carry it) is the smaller,
  better-supported change.

This lifts the sibling task's out-of-scope carve-out ("...should it dock
first, then act? — unless ideation finds the current behavior is actively
wrong, not just unspecified"): the trace above shows the current behavior
*is* actively wrong (misrouted `FocusPane`, not just an unspecified
click), so deciding the new behavior here is in scope.

### Implementation sketch (for the build stage)

Extend `decide_rail_click`'s existing signature with the `cols` it
already needs to reuse — no new pure function, no new constant:

```rust
fn decide_rail_click(
    line: isize,
    cols: usize,
    rows: &[Row],
    sessions: &[SessionEvent],
    gates: &[GateEvent],
    cwds: &BTreeMap<u32, PathBuf>,
) -> ClickAction {
    if cols < STATUS_MIN_COLS {
        return ClickAction::ToggleDock;
    }
    // ...existing section_layout(...).target(line) match, unchanged...
}
```

`handle_click` passes `self.last_cols` (already tracked, set every
`render()` call at `main.rs:633`) instead of adding a second gate or a
new field. Reusing `STATUS_MIN_COLS` keeps one width concept for
render/poll/click, matching the sibling task's own reasoning for reusing
the same constant rather than adding a sliver-specific one. No
`SectionLayout`/`target_for_line` change at all — the sliver path never
reaches them.

## Acceptance criteria

**AC-1 — Any click received while the rail is a sliver toggles dock,
never `FocusPane`/`FloatGate`/`None`.**
Verified by (offline, new test): for each of several row/session/gate
fixtures (0 rows; 5 rows; 2 rows + 1 session + 1 gate), call
`decide_rail_click(line, cols, ...)` with `cols < STATUS_MIN_COLS` for
every `line` from `-1` through a bound past the fixture's total item
count; assert every result is `ClickAction::ToggleDock`.

**AC-2 — The previously-misrouted lines are the ones now fixed (the
value-measuring AC).**
Verified by (offline, new test, against the 5-pane fixture traced above):
before the fix, `decide_rail_click` at `cols < STATUS_MIN_COLS` returns
`ToggleDock` for line 1, `FocusPane(row 0)` for lines 2-3, and
`FocusPane(row 1)` for lines 4-5 — write this as the pre-fix assertion,
confirm it's true against the current `main` (red for the *new*
behavior, i.e. this documents the bug), then change the assertion to
`ClickAction::ToggleDock` for all 5 lines and confirm it only passes once
the `cols` gate lands. This is the AC that measures the actual defect
found in tracing, not just the mechanism.

**AC-3 — Docked-width click behavior is unchanged.**
Verified by: `decide_rail_click`'s existing `cols >= STATUS_MIN_COLS`
match arms are moved behind the new gate with zero textual changes, so
every existing click test (`header_clicks_anywhere_toggle_the_swap_layout`,
`session_row_click_focuses_only_when_bound`, `gate_click_floats_tui_on_brief`,
`bad_log_suffix_never_floats`, etc.) passes with only their call sites
updated to pass an explicit `cols >= STATUS_MIN_COLS` (e.g. `DOCKED_COLS`)
— `cargo test` full-suite green, no assertion's expected value edited.

## Test plan

Entirely offline. `decide_rail_click`/`section_layout`/`target_for_line`
are pure functions over row/session/gate counts, a line number, and now
`cols` — no live zellij session is needed. Smallest-first ordering (the
riskiest unproven bit goes first, per TDD):

1. Write AC-2's pre-fix assertion against current `main` first and
   confirm it passes today (i.e. confirm the trace above is accurate,
   not a mis-reading of the source) — this is the "reproduce the bug"
   step, done before any code changes.
2. Flip AC-2's assertion to the fixed behavior; confirm it fails (red).
3. Implement the `cols` gate (the sketch above); confirm AC-1/AC-2 go
   green and update the existing docked-click tests' call sites for the
   new parameter (AC-3).
4. Full `cargo test` green, 0 pre-existing assertion values edited.

No interactive ACs: this task's entire surface is pure line/count → enum
mapping, identical in shape to the sibling task's own (fully offline)
click tests. A live spot-check (toggle to the sliver in a real session,
click it, confirm the rail expands) is a cheap optional final confirmation,
not required to prove the mechanism.

## Out of scope

Any new "remember which sliver line was clicked and auto-focus that item
once docked" behavior — dock-then-act means dock *only*; a fancier
carry-the-click-forward interaction is a new mechanism this task's
sprint exit criterion does not need and nothing in the trace above shows
is required to fix the misrouting. Nav-mode row highlighting and keyboard
navigation at sliver width — unaffected, out of scope per the sibling
task and still true here (no key handling is touched). The header/`⇄`
control's own docked-width behavior — unchanged, still literally the same
`ToggleDock` action, just now also the sliver's only click outcome.

## Stage Report: ideation

- DONE: Trace `target_for_line`/`section_layout`'s line-counting math against `sliver_lines`' output shape and determine whether existing click math still works by coincidence, is silently broken, and which rows if any are unclickable or misrouted.
  Read `handle_click`/`decide_rail_click`/`section_layout`/`target_for_line`/`sliver_lines` (`main.rs:631-1975`); found `handle_click` never consults `self.last_cols`, so every click always runs the docked 2-lines-per-row math against the sliver's 1-line-per-row output. Not coincidental — actively broken: line 1 is swallowed as a phantom header (`ToggleDock` instead of acting on row 0), pairs of sliver lines collapse onto one docked row index, and a 5-pane fixture shows every one of 5 clicks resolving wrong (3 of 5 rows entirely unreachable). See Proposed approach's "Trace" subsection for the full per-line table.
- DONE: Determine what the sliver's click behavior should be, and name whether the fix is offset math or something more.
  Decided dock-then-act, not corrected offset math: `sliver_lines` renders bare state glyphs with no title, so same-state items are visually indistinguishable even with correct offsets, and the sibling task's design intent frames the sliver as glance-only. See "Decide" subsection for the two supporting arguments (indistinguishable glyphs, prior design intent) and the lifted out-of-scope carve-out.
- DONE: Design concrete ACs and an offline test plan once the above is answered.
  AC-1 (every sliver click is `ToggleDock`), AC-2 (the value-measuring AC: the previously-misrouted 5-pane lines are exactly the ones now fixed), AC-3 (docked behavior unchanged, existing tests updated only at call sites) — all offline, pure-function tests extending `decide_rail_click`'s existing signature with a `cols` parameter reusing `STATUS_MIN_COLS`. See Acceptance criteria and Test plan sections.

### Summary

Traced the click-dispatch math against the sliver's actual output shape and found the sibling task's open question resolves to "actively broken," not "unspecified": `handle_click` ignores render width entirely, so docked line-counting assumptions misroute or swallow every sliver click. Decided dock-then-act over corrected direct-dispatch offsets, because the sliver's bare-glyph rendering makes same-state items visually indistinguishable regardless of offset correctness, and because the design's only prior sliver interaction was the header's dock-toggle. Proposed the smallest implementation (extend `decide_rail_click` with a `cols` parameter, gate on the existing `STATUS_MIN_COLS`, no new function or constant) and three offline ACs, including a value-measuring AC built directly from the concrete misroute table found during tracing.
