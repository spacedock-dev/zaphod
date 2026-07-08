---
id: ehvya2cwpsk28m8thtncbxe2
title: Floating sidebar-instance leak and dirty-tab chrome misplacement — suspected shared root cause
status: backlog
source: finding — spun out of dock-toggle-restructures-panes ideation (AC-2, AC-4), 2026-07-08
started:
completed:
verdict:
score: 0.85
worktree:
issue:
pr:
mod-block:
---

## Problem

Two live, independently-confirmed defects surfaced during ideation of
`dock-toggle-restructures-panes` (AC-2 and AC-4 there), spun out here because
neither is understood well enough yet to fix, and a plausible shared root
cause ties them together:

**1. Floating sidebar-instance leak (AC-2 finding).** Read-only
`dump-layout` of CL's live `WORK` session found 62 stray floating
`zellij-sidebar.wasm` panes accumulated across its tabs — 14 in "Noteplan",
47 in "CEO", 1 in "Tab #6", 0 in "GTM". Each is a config-matched (`rail
"1"`) bootstrap instance that never got promoted to a tiled resident and
never closed itself — `is_stray_floating_bootstrap`
(`src/main.rs:1375-1384`) only fires once its own tab holds a *tiled*
sidebar, which these tabs never got. Zellij's shared log
(`/tmp/zellij-501/zellij-log/zellij.log`) shows repeated `No such file or
directory` bursts for the sidebar wasm path at points across the session's
lifetime, consistent with a toggle firing while `./build.sh` was
mid-rebuild — a plausible contributor, though the log is shared across
other concurrently running sessions/worktrees building the same plugin, so
the correlation is suggestive, not conclusive.

**2. Dirty-tab chrome misplacement (AC-4 finding).** Live in the same
`WORK` session, tab "Tab #6": rail docked, CL closed one of the tab's other
panes (dirtying the swap layout), then toggled. Result, captured via
`dump-layout`: `zellij:tab-bar` landed as a `pane size="50%"
borderless=true { plugin location="zellij:tab-bar" }` **sibling inside the
vertical split** instead of its canonical `pane size=1 borderless=true` top
row — visibly breaking the tab bar (cramped into ~40-50% width, nothing
correctly below it). `status-bar` stayed correctly placed. A static trace
of `extract_chrome_panes` (`src/main.rs:1613-1667`) against the reported
shape found no obvious classification bug — the 3-line multi-line-child
form is exactly the case that function's branch is built to catch. Four
repro attempts in a clean, single-sidebar-instance disposable session (dock
→ add panes → toggle; close-to-2 → toggle; close-to-1 → toggle; a
`command=`/`start_suspended` pane matching CL's `spacedock` pane's shape →
toggle twice) **all toggled cleanly** — none reproduced the defect.

**Suspected shared root cause (unconfirmed).** `WORK`'s corrupted tab lives
in a session already carrying dozens of leaked floating instances (finding
1) in other tabs; the disposable sessions that failed to reproduce finding
2 only ever had one sidebar instance alive. `docs/docking-approach.md`'s
"Tab ids vs positions" section already documents a stale `tab_id →
position` translation race under load, and the plugin's "relaxed election"
lets any instance that perceives a sidebar-less or dirty active tab act on
it. A leaked zombie elsewhere racing a legitimate regenerate on the same
tab — each building its `override_layout` transform from a dump taken at a
slightly different instant, both writing to the same tab — is a plausible
way to corrupt chrome placement that neither actor's transform alone would
produce, and would explain why single-instance sessions can't reproduce it.
Not verified; this is the leading lead, not a conclusion.

## Proposed approach

Run a concurrency-focused spike as the first step, since it's the one
variable finding 2's four failed repro attempts didn't vary:

1. Reproduce finding 1 deliberately in a disposable session first (a known
   trigger, even approximate, for the leak) — candidate: fire `Alt /`
   repeatedly during a `./build.sh` rebuild window (`skip_cache true`
   forces a fresh compile per launch) and confirm stray floating instances
   accumulate the way they did in `WORK`.
2. With multiple live floating instances confirmed present in the same
   tab/session (not a clean single-instance session), reproduce finding 2's
   trigger sequence (dock → close a pane → toggle) and check whether the
   corruption now appears only when 2+ instances are live, confirming or
   refuting the race hypothesis directly rather than by inference.
3. Read `docs/docking-approach.md`'s "Tab ids vs positions" section in full
   — it may already document the exact race window rather than this being
   new territory.
4. Depending on what the spike finds: either (a) close the race at its
   source (idempotent/locked `override_layout`, or a stricter election that
   rules out a zombie acting on a tab it doesn't own), or (b) if the leak
   and the corruption turn out to be unrelated, root-cause each
   independently — but exhaust the shared-cause hypothesis first since it's
   the cheapest single spike that could resolve both findings at once.

## Acceptance criteria

**AC-1 — The floating-instance leak's trigger is reproduced on demand and
either fixed or bounded.**
Verified by: a disposable-session repro that reliably produces at least one
stray floating instance (not a hypothesis), plus a fix (preventing the leak)
or a bound (a cleanup sweep, a cap, or a self-terminating timeout for an
unpromoted floating bootstrap) — whichever the spike's root cause supports.

**AC-2 — The dirty-tab chrome-misplacement defect is reproduced on demand.**
Verified by: a disposable-session repro using the concurrency setup from
the Proposed approach that triggers the tab-bar relocation into the content
region at will, not just the single live sighting in `WORK`.

**AC-3 — Chrome placement cannot be corrupted by a concurrent
regenerate/retrofit race, regardless of how many stray instances exist.**
Verified by: a test or live repro showing that with AC-1's leak trigger
reproduced (multiple live instances) and a legitimate regenerate firing
concurrently, `extract_chrome_panes`/`override_layout`'s output still places
`tab-bar`/`status-bar` in their canonical rows every time — not just when
the leak happens to be absent.

## Test plan

Riskiest first, and it's the same step for both findings: the concurrency
repro (AC-1's leak trigger, then AC-2's corruption trigger on top of it).
Nothing else in this entity can be verified until that repro exists — four
single-instance attempts already ruled out the non-concurrent case in the
parent entity's ideation pass, so re-attempting without concurrency would
waste the spike.

## Out of scope

The first-toggle pane-count invariant (AC-1/AC-2/AC-3 of
`dock-toggle-restructures-panes`) — already ideated and documented there,
unaffected by this entity's findings. Do not re-litigate that decision here.
