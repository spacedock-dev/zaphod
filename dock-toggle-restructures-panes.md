---
id: j5zfk59gesvg0zfyw23haf56
title: Dock toggle restructures panes on first activation — contradicts docking-approach.md invariant
status: backlog
source: finding — CL live session, Alt-/ dock regression, 2026-07-08
started:
completed:
verdict:
score: 0.8
worktree:
issue:
pr:
mod-block:
---

## Problem

`docs/docking-approach.md:266-270` and `SPEC.md`'s landmine #16 both state the
toggle (`Alt /` / the header `⇄` control) "never creates, hides, shows,
moves, or destroys a pane" — it only cycles a tab's `swap_tiled_layout`
states. That invariant is false for the *first* toggle in a tab that has no
rail yet: `decide_toggle()` picks `Retrofit`, which calls
`install_split_preserving_swaps()` -> `override_layout()`. That dumps the
tab's current pane tree and reinstalls it wrapped in a new
`pane split_direction="vertical" { rail; region }` — inserting the rail as a
brand-new sibling pane. A tab with 1 pane before the first toggle has 2
after. CL hit this live in the `WORK` daily-driver session on 2026-07-08.

This is a regression, not a pre-existing quirk the docs failed to capture:
v1-v3 of this plugin (commits `95e0c25`..`d6ad89b`, 2026-06-10 to 06-20) only
ever hid/showed/floated/resized the sidebar's own pane, never touching the
user's panes or tabs. The invasive retrofit mechanism was introduced at
commit `3154259` ("Toggle via swap cycling; retrofit tabs without a
sidebar", 2026-07-02) and hardened by every commit since, but the docs were
never corrected to describe a first-toggle exception — they still promise
the old invariant unconditionally, and HEAD (`a03a20e`) still ships the
regression.

Related, confirmed live during the same session: once a tab is docked,
manually moving/adding a pane into it does not join the rail's tracked
`region` split — zellij just marks the tab's swap layout dirty
(`is_swap_layout_dirty` / `swap_dirty` in the plugin). The rail doesn't
reincorporate the new pane until the *next* `Alt /` press, at which point
`decide_toggle`'s `active_swap_dirty` branch fires `RegenerateSwaps`, which
funnels into the same `install_split_preserving_swaps()` and rewraps
*everything currently in the tab* (including whatever was just moved in)
into a fresh docked/undocked pair — i.e. the same full pane-tree rewrite
happens again, silently, on that next toggle.

Also open and unexplained: pressing `Alt /` in `WORK` additionally appeared
to open a brand-new tab. No `new_tab` call exists anywhere in `src/main.rs`
by static reading of the retrofit/toggle code path — this needs its own
root-cause (candidate: `override_layout`'s behavior when called from a
still-floating, not-yet-tiled plugin instance) before a fix is designed,
since it may be a distinct bug from the pane-split one.

## Proposed approach

Ideation should settle two decisions and root-cause the open question,
before implementation starts:

1. Root-cause the new-tab creation observed live — confirm or rule out
   `override_layout` (or another host call in the retrofit path) creating a
   new tab when invoked from a floating (not-yet-tiled) plugin instance,
   rather than relaying out the active tab in place.
2. Decide the fix shape for the pane-split-on-first-toggle regression:
   either (a) restore a genuinely non-invasive path for the zero-rail case —
   dock by floating/showing the rail without wrapping the tab's existing
   panes in a new split, closer to the pre-`3154259` v1-v3 mechanism — or
   (b) if wrapping into a swap-layout set is structurally required for the
   swap-cycling design to work at all, correct `docs/docking-approach.md`
   and `SPEC.md` to state the first-toggle (and dirty-tab-regenerate)
   exception precisely, and make the actual pane-count change an explicit,
   reviewed AC rather than an undocumented side effect.
3. Whichever shape is chosen, the same fix must cover the dirty-tab
   `RegenerateSwaps` path — it re-triggers the identical full rewrite on any
   toggle after a manual pane move, not just on the very first dock.

## Acceptance criteria

**AC-1 — First dock into a bare tab does not change pane count.**
Verified by: a fixture/test that starts a tab with N panes (N >= 1, no
existing rail), fires the toggle, and asserts the tab still has exactly N
user panes plus at most the rail (no `region`-wrapping pane materializes
where none existed), or an explicit, CL-approved doc correction if (b) is
chosen instead — in which case this AC is replaced by an AC asserting the
docs accurately describe the actual mechanism.

**AC-2 — New-tab creation is root-caused and either eliminated or explained.**
Verified by: a live repro in a fresh zellij session showing whether a new
tab appears on first toggle; if it does, a fix or a documented reason it is
correct behavior.

**AC-3 — Manual pane moves into a docked tab do not trigger a full
pane-tree rewrite.**
Verified by: a fixture/test reproducing the dirty-tab `RegenerateSwaps` path
and asserting it does not re-wrap panes the user didn't just add (or,
depending on the direction chosen in AC-1, that any rewrap is the agreed
documented behavior, not a silent side effect).

## Test plan

Riskiest-first: reproduce the first-toggle pane-split in a disposable
fixture/test (fast, offline) before touching any live-demo mechanics — this
is a `retrofit`/`decide_toggle` unit-test-level bug, not one that requires a
live zellij session to pin down. The new-tab mystery (AC-2) does need a live
zellij session spot-check since it wasn't reproducible via static code
reading alone.

## Out of scope

Redesigning the swap-cycling docking architecture wholesale, or re-opening
the "runtime tiled docking is unwinnable" question `docs/docking-approach.md`
already settled — this task is about the undocumented pane-restructuring
side effect and the new-tab mystery, not the overall docking approach.
