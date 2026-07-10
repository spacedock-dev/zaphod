---
id: fwej8kfqk7gavmnpe33mfgjs
title: Repeated Alt-/ presses on an already-docked tab never fully undock — deterministic 3-press dock/no-op/sliver-collapse cycle
status: backlog
source: finding — e2e scenario suite (workflow wf_a948b6c2-3bf), spun out of dock-floating-leak-and-chrome-misplacement's ideation cycle 5, 2026-07-10
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

Repeatedly pressing `Alt /` on a single already-docked tab does not toggle
the sidebar between docked and undocked as the header `⇄` control and the
architecture doc both imply. Instead it follows a deterministic 3-press
cycle that never returns to the sidebar-absent state:

1. press 1 — full 28-col dock (expected)
2. press 2 — silent no-op (sidebar unchanged, no observable effect)
3. press 3 — sidebar collapses to a 1-column sliver (not fully undocked,
   not absent — a third, intermediate state)
4. press 4 — back to full 28-col dock, and the cycle repeats

Confirmed **100% reproducible across two independent clean sessions** (14
total presses, `list-panes -a` snapshotted after every single press) run as
part of `dock-floating-leak-and-chrome-misplacement`'s e2e scenario suite
(`docs/e2e-scenarios.md`, scenario 2 "Toggling back and forth on an
already-docked tab", commit `9f9a0b2` on
`.worktrees/spacedock-ensign-dock-floating-leak-and-chrome-misplacement`).
A second, independent run (scenario 3, the dirty-tab case) corroborated the
same no-op-on-second-press behavior as a side effect.

**Chrome placement is unaffected** — `zellij:tab-bar`/`zellij:status-bar`
stayed correctly pinned at their canonical `size=1` top/bottom rows through
all 14 presses in both runs; this is not a manifestation of
`dock-floating-leak-and-chrome-misplacement`'s chrome-relocation bug. It was
originally surfaced while investigating a CL-reported live symptom
("`Alt /` sometimes turns the tab fullscreen and hides the top tab bar,
non-deterministically") — that specific symptom was NOT reproduced under
these controlled conditions (no fullscreen, no hidden chrome, in either
run), so this is very likely a distinct, previously-undiscovered defect
surfaced by the same investigation, not a confirmed explanation of CL's
original report. CL will provide more detail if the fullscreen symptom
recurs.

Root cause not yet investigated. Per this entity's own live-testing lead
(`dock-floating-leak-and-chrome-misplacement` ideation cycle 4's addendum),
the likely code path is `decide_toggle`'s `SteerSwap`/cycle branch — which
calls zellij's own `next_swap_layout()`/`previous_swap_layout()` directly,
with no plugin-side KDL construction — as distinct from the `Retrofit`/
`RegenerateSwaps` paths `dock-floating-leak-and-chrome-misplacement` has
been investigating. If confirmed, this would be a defect in how the plugin
drives zellij's swap-layout engine (e.g. an off-by-one in which swap
position it thinks it's cycling to, or the `BASE`-swap-position mechanic
`docs/docking-approach.md`'s Toggle v2 evidence already flagged as a
source of nondeterministic cycle length), not the same install-time race
`dock-floating-leak-and-chrome-misplacement` tracks.

## Out of scope

Root-causing or fixing `dock-floating-leak-and-chrome-misplacement`'s
floating-instance leak, install-time race, or chrome-misplacement findings
— this is a related but distinct symptom in a different code path
(`SteerSwap`, not `Retrofit`/`RegenerateSwaps`), filed separately per CL's
explicit direction. Confirming or explaining CL's originally-reported
fullscreen/tab-bar-hidden symptom — not reproduced here; revisit only if
CL provides a fresh repro with more detail.
