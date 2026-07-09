---
id: 865qt0sj9zrmg75vwv2z598r
title: 1-col undocked sliver renders unreadable wrapped text instead of a compact indicator
status: implementation
source: finding — live session dogfooding, 2026-07-08/09
started: 2026-07-09T07:14:24Z
completed:
verdict:
score: 0.6
worktree: .worktrees/spacedock-ensign-sliver-rendering-lacks-narrow-width-handling
issue:
pr:
mod-block:
---

## Problem

The rail's `render()` (`src/main.rs:631`) has no width-conditional branch at
all — it prints the same content whether the rail is docked (28 cols) or
collapsed to the undocked 1-col sliver. Two things bypass the existing
`cols.saturating_sub(N)` truncation entirely:

1. **Section headers are literal, unconditional strings.** `"▾ GATES"`
   (`main.rs:678-681`) and `"▾ AGENTS"` (`main.rs:665-668`) print their full
   text regardless of `cols` — only the trailing padding spaces get
   truncated. At 1 column wide, the terminal wraps the literal string
   character-by-character down the column.
2. **State-marker glyphs are prepended before any truncation.** Every row's
   colored dot (`state_glyph`/`row_marker`, e.g. `session_row_line`
   `main.rs:1918-1930`, `gate_row_line` `main.rs:1934-1941`) is prepended
   unconditionally — never gated on `cols`.

The same literal-string defect also applies to the third, ever-present
header — `"▾ PANES...⇄"` (`main.rs:647-650`) — not separately numbered
above only because it isn't the glyph CL's live symptom named below; AC-1's
"or any other multi-character literal" clause covers it, and the fix
suppresses it identically to the other two headers.

Live symptom (CL, 2026-07-08/09): the 1-col sliver shows a down-arrow glyph
followed by "G", "A", "T", "E", "S" each wrapped onto its own row (the
literal `"▾ GATES"` string wrapping character-by-character), followed by a
column of red dots with no discernible structure — unreadable, and not
what a minimal-width "at a glance" indicator should look like.

## Proposed approach

### Threshold: reuse `STATUS_MIN_COLS = 8`, no new constant

Both real widths this plugin ever renders at are fixed by the swap-layout
mechanism: `DOCKED_COLS = 28`, `UNDOCKED_COLS = 1` (`main.rs:32-33`) — there
is no continuum of intermediate widths in normal operation, so the exact
numeric cutover is not load-bearing precision; it only has to separate 1
from 28, and 8 already does that with margin on both sides. `STATUS_MIN_COLS`
is already this codebase's dividing line between "wide enough for real
content" and "too narrow" (`main.rs:34-36`'s own comment: "too narrow for
status text"). A second, sliver-specific constant with the same effective
boundary (any value 2-27 behaves identically today) would be two names for
one fact — introduces a distinction with no behavioral difference, and two
knobs that could drift apart for no reason. Reusing it also makes an
implicit coupling explicit and correct: `should_poll_statuses` already
stops polling below this width because there's no room to show status text
(`main.rs:1237-1253`); gating the render mode on the same constant means the
rail never polls for a width at which it no longer renders that text, and
vice versa — one width concept, not two.

### Direction: (a) markers-only, stacked — chosen over (b) aggregate

- **(a) Markers only, no text.** Suppress all section-header strings and
  free text (titles/status/summary) below the threshold; render just the
  stacked state-marker glyphs, one per row/session/gate, in original
  top-to-bottom order.
  - *For:* preserves per-item state, not just "something is wrong" — a
    fixture with 3 blocked gates out of 10 rows still shows 3 distinct
    dots, not 1. Reuses the exact glyph values normal rendering already
    computes (`row_marker`, `state_glyph`, the same `Blocked` marker
    `gate_row_line` uses for gates) — the compact mode never invents a new
    visual, it only omits text. No new state-precedence concept to design,
    document, or get wrong.
  - *Against:* with many concurrent rows the glyph column can exceed the
    pane's visible height — but that is already true of full-width
    rendering today (not a regression this mode introduces), and typical
    single-tab agent/session/gate counts are small (1-10).
- **(b) Single aggregate indicator.** Collapse everything below the
  threshold to one glyph reflecting the worst state present anywhere
  (`blocked > working > idle > done/unknown`).
  - *For:* truest "one glance," simplest possible output shape.
  - *Against:* loses count and identity entirely — 1 blocked row and 10
    blocked rows render identically, which is a real regression from what
    a "column of dots" already half-conveys today per CL's live symptom.
    It also requires inventing and encoding a severity ordering that exists
    nowhere else in the codebase (is `Idle` — an agent waiting on the
    user — really less urgent than `Working`? not obviously), a new
    product judgment call this task doesn't need to make to fix the
    reported bug.
- **Decision: (a).** It is the smaller change (an early-return branch plus
  one small pure function reusing existing glyph logic, vs. inventing and
  justifying a severity order), it strictly preserves more signal, and it
  matches CL's own framing of the value bar ("the closest thing to the same
  information, minimally") without committing to an unvalidated product
  opinion about which state matters most. (b) is not implemented, but
  nothing here forecloses revisiting it once real narrow-width usage shows
  (a)'s row count is actually a problem.

### Implementation sketch (for the build stage)

Add one pure function beside `state_glyph`/`row_marker`
(`main.rs:1902-1954`), reusing them rather than recomputing glyphs:

```rust
// Compact 1-glyph-per-row rendering below STATUS_MIN_COLS: no header text,
// no titles/status/summary — one state marker per row, session, and gate,
// in the same top-to-bottom order render() prints them in. Reuses the same
// glyph functions the normal-width render calls, so the compact mode never
// shows a color/symbol the normal mode wouldn't have shown for that item.
fn sliver_lines(rows: &[Row], sessions: &[SessionEvent], gates: &[GateEvent]) -> Vec<String> {
    rows.iter()
        .map(|row| row_marker(row).trim_end().to_owned())
        .chain(sessions.iter().map(|session| {
            state_glyph(agent::marker_for_state(&session.state)).trim_end().to_owned()
        }))
        .chain(gates.iter().map(|_| state_glyph(agent::AgentState::Blocked).trim_end().to_owned()))
        .collect()
}
```

`render()` gains one early-return branch before its current body, which
stays byte-for-byte unchanged (this is the concrete mechanism behind AC-3):

```rust
if cols < STATUS_MIN_COLS {
    for line in sliver_lines(&self.rows, &self.sessions, &self.gates) {
        println!("{line}");
    }
    return;
}
```

`.trim_end()` drops `state_glyph`'s trailing spacer space (present so the
normal render has a gap before the title) — at 1 column that trailing space
has nothing to precede and would itself wrap to a blank line; trimming it
keeps each sliver line to exactly one visible glyph (or, for
`AgentState::Unknown`'s all-space marker, an empty line — no visible
indicator for a state that has none today either).

## Acceptance criteria

**AC-1 — No section-header text (or any other multi-character literal)
renders at sliver width.**
Verified by (offline, new test in `main.rs`'s `mod tests`): a fixture with
a distinctively-titled row, session, and gate (e.g. titles/agent/summary
containing strings like `"distinctive-pane-title"`) rendered through
`sliver_lines` at/below the threshold; assert the joined output contains
none of `"PANES"`, `"AGENTS"`, `"GATES"`, `"⇄"`, or any of the fixture's own
title/agent/summary/detail strings, and assert each returned line equals
exactly the corresponding `row_marker(...)`/`state_glyph(...)` value
(trimmed) — an exact-equality check, not a substring guess, so no
additional text can ride along undetected.

**AC-2 — The sliver still communicates real state, not just fewer bytes.**
Verified by (offline, new test): `sliver_lines` on an all-`Idle` row
fixture vs. an otherwise-identical fixture with one row switched to
`Blocked` — assert the two outputs differ (`assert_ne!`) and that the
changed row's line equals `state_glyph(AgentState::Blocked)` (trimmed)
while the unaffected rows' lines are unchanged — proves the compaction
tracks real state per row, not just a blanket "fewer bytes."

**AC-3 — Docked (normal-width) rendering is completely unaffected.**
Verified by: `render()`'s existing body (the `cols >= STATUS_MIN_COLS`
path) is moved with zero textual changes behind the new early-return guard
— reviewable as a pure insertion in the build stage's diff — so every
existing test that exercises the functions it calls (`row_marker`,
`session_row_line`, `gate_row_line`, `section_layout`, `status_line`, etc.)
passes unmodified; `cargo test` full-suite green with no pre-existing test
edited is the concrete check.

## Test plan

Entirely offline — `sliver_lines` (like `row_marker`/`session_row_line`/
`gate_row_line`/`state_glyph` before it) is a pure function of `Row`/
`SessionEvent`/`GateEvent` fixtures; no live zellij session is needed to
prove AC-1/AC-2, and AC-3 is proved by the existing suite staying green. A
live spot-check (toggle to the sliver in a real session, eyeball the
column of dots) is a cheap final confirmation, not required to prove the
mechanism — the riskiest unproven bit (does `sliver_lines`'s output
actually read as "no wrapped artifacts" once the terminal really is 1
column wide) is exactly what AC-1's exact-equality check settles without
needing a live pane.

## Out of scope

Redesigning the docked (normal-width) layout — unaffected by this task.
Fixing the floating-sidebar leak or chrome-misplacement bugs (tracked
separately in `dock-floating-leak-and-chrome-misplacement`) — unrelated to
this rendering gap even though both surfaced in the same session.
Click-target line mapping (`SectionLayout`/`target_for_line`) at sliver
width — the compact renderer's line count (1 line per item, no headers) no
longer matches the docked-width row-to-line assumptions baked into click
dispatch; whether/how mouse clicks (e.g. the header's dock-toggle click)
should still work on a 1-col sliver is a separate concern, not addressed
here. Nav-mode row highlighting at sliver width — nav's own key handling is
unaffected by this task, and the compact renderer draws every row
undecorated (selected or not) since there is no title left to highlight.

## Stage Report: ideation

- DONE: Decide and justify the narrow-width threshold
  Reuse `STATUS_MIN_COLS = 8` — see Proposed approach's Threshold subsection; no new constant added.
- DONE: Weigh markers-only vs. aggregate indicator, pick one with reasoning
  Chose (a) markers-only stacked over (b) aggregate — see Proposed approach's Direction subsection for tradeoffs.
- DONE: Design AC-1/AC-2/AC-3 into concrete offline render tests
  Each AC rewritten with a named fixture shape and exact-equality assertions against `sliver_lines`; see Acceptance criteria section.

### Summary

Read `render()` (`main.rs:631-687`), the existing marker/line pure functions (`row_marker`, `state_glyph`, `session_row_line`, `gate_row_line`), and `SPEC.md`'s own test-pyramid guidance ("pure-function unit tests... extend the pattern") to design a narrow-width fix that adds one small pure function (`sliver_lines`) and one early-return branch in `render()`, rather than rewriting the docked-width path. Key decision: markers-only over aggregate-indicator, because it preserves per-row signal and requires no new severity-ordering concept. Also flagged (in Problem and Out of scope) two things the original checklist didn't name: the `"▾ PANES...⇄"` header shares the same defect and is covered by the same fix, and click-target line mapping at sliver width is a real follow-on question this entity does not resolve.

## Stage Report: implementation

- DONE: Add the sliver_lines pure function and the early-return branch in render() exactly as sketched in ideation, reusing row_marker/state_glyph verbatim -- the existing cols >= STATUS_MIN_COLS body must move behind the guard with zero textual changes.
  `src/main.rs` — `sliver_lines` added at commit 40af368 (beside `row_marker`); early-return guard added at commit f83fd90, a single 6-line diff hunk with no other line touched.
- DONE: Write AC-1/AC-2/AC-3's tests as designed: exact-equality checks against sliver_lines output (no header/title/summary substrings, marker values match trimmed row_marker/state_glyph exactly), the Idle-vs-Blocked differential test, and confirm the full existing suite passes unmodified.
  `sliver_lines_carry_no_header_or_free_text` (AC-1) and `sliver_lines_differ_when_one_row_state_changes` (AC-2) both red before the fix — `error[E0425]: cannot find function 'sliver_lines' in this scope` — then green after; full suite 132/132 passing (130 pre-existing + 2 new, 0 failed, 0 edited).
- DONE: Confirm the "PANES...⇄" header is also suppressed by the same early-return guard (it's covered structurally, not by a separate fix) and record that confirmation in the stage report.
  `src/main.rs:645-650`'s `if cols < STATUS_MIN_COLS { ...; return; }` sits directly before line 653's `▾ PANES` println and before the `▾ AGENTS`/`▾ GATES` printlns (670/683) — one guard returns ahead of all three headers, no separate fix needed.

### Summary

Added `sliver_lines` reusing `row_marker`/`state_glyph` verbatim (commit 40af368), then gated `render()` on `STATUS_MIN_COLS` with a single early-return branch ahead of the existing body (commit f83fd90), leaving that body textually untouched. Both new tests were confirmed red (compile failure: `sliver_lines` undefined) before implementation and green after; `cargo test` (132/132) and `cargo check --tests` are clean, and the PANES/AGENTS/GATES headers are all suppressed by the one guard rather than three separate fixes.
