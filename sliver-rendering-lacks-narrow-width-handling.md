---
id: 865qt0sj9zrmg75vwv2z598r
title: 1-col undocked sliver renders unreadable wrapped text instead of a compact indicator
status: backlog
source: finding — live session dogfooding, 2026-07-08/09
started:
completed:
verdict:
score: 0.6
worktree:
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

Live symptom (CL, 2026-07-08/09): the 1-col sliver shows a down-arrow glyph
followed by "G", "A", "T", "E", "S" each wrapped onto its own row (the
literal `"▾ GATES"` string wrapping character-by-character), followed by a
column of red dots with no discernible structure — unreadable, and not
what a minimal-width "at a glance" indicator should look like.

## Proposed approach

Ideation should design a real narrow-width rendering mode, gated on `cols`
(candidate: reuse `STATUS_MIN_COLS = 8`, already the threshold this file
uses elsewhere to decide "is the rail usably wide," or pick a separate
sliver-specific threshold if 8 doesn't fit — decide and justify either
way). Weigh at least these directions rather than assuming one:

- **(a) Markers only, no text.** Suppress all section-header strings and
  free-text (titles/status/summary) below the threshold; render just the
  stacked state-marker glyphs, one per row, in original row order — the
  closest thing to "the same information, minimally."
- **(b) Single aggregate indicator.** Collapse everything below the
  threshold to one glyph reflecting the worst state present anywhere
  (blocked > working > idle > done/unknown) — true one-glance, loses
  per-row detail entirely.
- Note CL's own framing was an open question ("what would be more
  useful") — this is a real product decision, not a foregone conclusion;
  present both (or whatever ideation finds) with tradeoffs rather than
  silently picking one.

## Acceptance criteria

**AC-1 — No section-header text (or any other multi-character literal)
renders at sliver width.**
Verified by: a render test at the chosen narrow-width threshold asserting
the output contains no section-header substrings ("GATES"/"AGENTS") and no
wrapped-letter artifacts — an independent check on the rendered string
shape, not a re-read of the render function's own logic.

**AC-2 — The sliver still communicates real state, not just fewer bytes.**
Verified by: a test asserting the sliver's glyph output changes when the
underlying row/session/gate states change (e.g. a fixture with one Blocked
row produces a visibly different sliver than an all-Idle fixture) — proves
the compaction preserves signal, doesn't just blank the display.

**AC-3 — Docked (normal-width) rendering is completely unaffected.**
Verified by: the existing render/status-line test suite passes unmodified.

## Test plan

Entirely offline — `render()`'s output is a pure function of `self` state
and `cols`; no live zellij session needed to verify the string shape at a
given width. A live spot-check (toggle to the sliver in a real session) is
a cheap final confirmation, not required to prove the mechanism.

## Out of scope

Redesigning the docked (normal-width) layout — unaffected by this task.
Fixing the floating-sidebar leak or chrome-misplacement bugs (tracked
separately in `dock-floating-leak-and-chrome-misplacement`) — unrelated to
this rendering gap even though both surfaced in the same session.
