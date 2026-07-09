---
id: n5w8b1s9n8zqczvw2ebd25q8
title: Click-target line mapping is unverified at sliver width after the compact renderer
status: backlog
source: finding — named as out of scope by sliver-rendering-lacks-narrow-width-handling's ideation, 2026-07-09
started:
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

Ideation should trace `target_for_line`/`section_layout`'s line-counting
math against the new `sliver_lines` output shape (offset by width, i.e.
whether `render()`'s early-return branch needs its own click-mapping
counterpart, not just its own print path) and determine: (a) does the
existing click math happen to still work by coincidence, (b) is it
silently broken (which rows if any are unclickable or misrouted), and (c)
what the sliver's click behavior *should* be — clicking a stacked glyph
with no visible title is a different interaction than clicking a titled
row, and the fix may be more than "fix the line offsets."

## Acceptance criteria

Ideation defines these once (a)/(b)/(c) above are answered; do not assume
"just fix the offset math" is sufficient without confirming what's
actually broken first.

## Test plan

Offline: `target_for_line`/`section_layout`/`decide_rail_click` are pure
functions over row/session/gate counts and a line number — extend the
existing click-mapping test pattern to the sliver's line-count shape.

## Out of scope

Redesigning what should happen when a sliver row IS clicked correctly
(e.g. should it dock first, then act?) unless ideation finds the current
behavior is actively wrong, not just unspecified.
