---
id: j5zfk59gesvg0zfyw23haf56
title: Dock toggle restructures panes on first activation — contradicts docking-approach.md invariant
status: ideation
source: finding — CL live session, Alt-/ dock regression, 2026-07-08
started: 2026-07-08T07:03:01Z
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

Both open questions are settled by direct investigation (static reading plus
a live repro in a disposable session; see evidence below), not by further
brainstorming:

### Root cause: new-tab creation — REFUTED as a toggle-caused tab

Confirmed by two independent checks:

- **Static.** No `new_tab`/`NewTab` host call exists anywhere in the toggle
  path (`decide_toggle`, `perform_toggle`, `install_split_preserving_swaps`,
  `retrofit`, `regenerate_swaps`) — grep of `src/main.rs` for `new_tab` and
  `NewTab` turns up only an unrelated KDL fixture constant
  (`new_tab_template`, `src/main.rs:3725`) and its own test assertion.
- **Live, 2026-07-08.** Built HEAD (`a03a20e`) fresh via `./build.sh`,
  launched a disposable `zellij --session ztestx1` session inside a tmux pty
  (isolated from CL's live `WORK` session — never touched it except
  read-only `dump-layout`), and sent a real `Alt /` keypress
  (`tmux send-keys Escape "/"`, chorded by the terminal exactly like a real
  keyboard) to a fresh chrome-only tab with one shell pane. Before: 1 tab, 1
  user pane (`terminal_0`) plus builtin chrome/link plugins
  (`zellij action list-panes -a`). After: still 1 tab (`zellij action
  list-tabs` unchanged — `TAB_ID 0, POSITION 0`), but a new tiled plugin
  pane (`plugin_4`, the sidebar) appeared as a sibling of the user's pane in
  `dump-layout`'s `pane split_direction="vertical" { pane name="sidebar"
  size=28 { ... } ; pane focus=true }`. Zero new tabs, one new pane — the
  exact AC-1 mechanism, and no trace of a spawned tab.

The new-tab hypothesis named in the checklist ("`override_layout` called
from a still-floating, not-yet-tiled plugin instance creates a new tab") is
refuted: `override_layout` with `apply_only_to_active_tab=true` only ever
touched the one active tab in every run.

**What likely produced the "new tab" perception.** Read-only inspection of
the live `WORK` session (`zellij --session WORK action dump-layout` — a
non-mutating CLI action, no keys sent, nothing in that session touched or
disrupted) found no unexplained tab, but did find a severe, independent
leak: 62 stray floating `zellij-sidebar.wasm` panes piled up across the
session's tabs — 14 in tab "Noteplan", 47 in tab "CEO", 1 in the focused
"Tab #6", 0 in "GTM" (counted from the dump: `grep -c
zellij-sidebar.wasm`, split per tab, all under each tab's `floating_panes`
block, none in the tiled region). Each is a config-matched (`rail "1"`)
sidebar instance that never became a tiled resident and never closed
itself — `is_stray_floating_bootstrap` (`src/main.rs:1375-1384`) only fires
once its own tab holds a *tiled* sidebar, which these tabs never got. A
floating pane materializing over the user's current view is visually easy
to mistake for a new tab/window popping up, especially with dozens stacked
(`dump-layout`'s per-pane `x`/`y` offsets step by exactly +2 each,
confirming a pile, not a rendering artifact). Zellij's own log
(`/tmp/zellij-501/zellij-log/zellij.log`, shared across every concurrently
running session on the machine) shows repeated multi-attempt bursts of
`No such file or directory` for the sidebar wasm path at several points
across the session's lifetime, consistent with a toggle firing while
`./build.sh` was mid-rebuild — a plausible contributor, though the log is
shared across other concurrent sessions/worktrees building the same
plugin, so this correlation is suggestive, not conclusive. Attempting to
force-reproduce the leak directly (moving the wasm aside, firing `Alt /` at
a sidebar-less tab in the disposable session) did not spawn a fresh zombie
in the time available — the exact trigger needs a cleaner, single-session
repro with `debug "1"` tracing on, which is more than this ideation pass
should spend.

This leak is a distinct bug class (plugin-instance lifecycle/dedup) from
this entity's scope (the pane-restructuring invariant and the new-tab
mystery) and is **not** fixed here — see Out of scope. It fully answers the
root-cause checklist item (the "new tab" was not a tab), while leaving the
leak's exact trigger for a follow-on finding.

### Fix direction: document the first-toggle pane-count change — chosen over a non-invasive rewrite

Direction **(b)** from the original approach: correct
`docs/docking-approach.md` and `SPEC.md` to state the exception precisely,
and make the pane-count change an explicit, reviewed AC. Direction (a) — a
non-invasive dock path for the zero-rail case — is rejected, because it is
not available under the validated architecture, not merely undesirable:

- `docs/docking-approach.md`'s own "Outcome" and "Toggle v3" sections
  already ruled out every non-invasive alternative: the floating overlay
  (`float_as_rail`) was superseded specifically because it does not reserve
  space; hide/show (`show_self`) was disproven live (re-inserts at the
  wrong size/position); a blind stacked-absorb was deleted
  (`absorb_retrofit`) for destroying splits under the relaxed election.
- Reserving a tiled column on a tab that has none, at runtime, has exactly
  one validated mechanism in this codebase: `override_layout` with a base
  template that wraps the existing arrangement next to a new rail slot.
  Materializing that slot **is** materializing a pane — there is no
  variant of this mechanism that reserves space without adding a pane for
  it, so "restore a non-invasive path" would mean reopening the docking
  architecture question this entity's own Out of scope (and
  `docs/docking-approach.md`'s "runtime tiled docking" verdict) rules out.
- The dirty-tab `RegenerateSwaps` path reuses the identical
  `install_split_preserving_swaps` machinery, but its `dump_contains_sidebar`
  check (`src/main.rs:912-919`) means the *already-resident* rail is
  re-seated by identity (`retain_existing_plugin_panes=true`) rather than
  duplicated — a regenerate does not change pane count, only split-nesting
  fidelity can degrade (the accepted ceiling,
  `docs/docking-approach.md`'s "split-preserving fidelity ceiling" section,
  SPEC landmine #35). So AC-3's "full pane-tree rewrite" concern is already
  covered by that existing, live-validated ceiling — it needs a citation,
  not a new mechanism or a new AC.

## Acceptance criteria

**AC-1 — First dock into a bare tab installs the rail as a stated,
reviewed pane-count change, not a silent side effect.**
Verified by (offline): a fixture/test around `split_preserving_layout_kdl`
(`src/main.rs:1461`) that dumps a tab with N panes (N = 1 and N = 3) and
asserts the generated `docked`/`undocked` swap KDL always nests exactly one
rail pane as a sibling of the absorbed region — independent of N and
independent of the entity's own prose, since it inspects the KDL string
`install_split_preserving_swaps` actually hands to `override_layout`.
Verified by (interactive, already run 2026-07-08): a fresh 1-pane
disposable session went from 1 to 2 panes on the first `Alt /`, zero tabs
created (see Proposed approach). AC is satisfied once
`docs/docking-approach.md:266-270` and both `SPEC.md` sites (landmine #16;
"If building v2 from scratch" item 3) state this as accepted first-toggle
behavior instead of claiming the toggle never creates a pane
unconditionally (doc diff below).

**AC-2 — New-tab creation is root-caused: refuted as a toggle-caused tab,
and the more plausible source is named.**
Verified by (offline): grep confirms no `new_tab`/`NewTab` host call in the
toggle path. Verified by (interactive, already run 2026-07-08): live repro
in a disposable session shows tab count unchanged (1) through the first
toggle; read-only `dump-layout` of the live `WORK` session shows no
unexplained tab, but does show 62 stray floating sidebar panes across three
of its four tabs — a live, independently-confirmed leak that is the
leading candidate for what looked like "a new tab." Filing the leak's own
root cause and fix is out of scope for this entity (see Out of scope /
Follow-on finding).

**AC-3 — Manual pane moves into a docked tab do not change pane count on
a regenerate; only documented nesting-fidelity loss can occur.**
Verified by: existing coverage — `install_split_preserving_swaps`'s dump
abort (`src/main.rs:912-919`) plus `retain_existing_plugin_panes=true`
already re-seat the resident rail by identity rather than duplicating it on
a dirty-tab regenerate, and the one known degradation mode (split-nesting
flattening on a rail-width flip, not pane count) is already live-validated
in `docs/docking-approach.md`'s fidelity-ceiling section and SPEC landmine
#35. No new test or mechanism is needed; the fix is the doc diff below
citing the ceiling from the toggle invariant directly.

## Proposed doc diff

**`docs/docking-approach.md:266-270`** — replace the unconditional
invariant with a scoped one and a new exception paragraph:

```diff
 **The sidebar pane exists in every toggled tab's layout, permanently.** The
 default layout is chrome-only; a tab's first toggle retrofits the sidebar in.
-"Toggle" (`Alt /` and the `⇄` header control) never creates, hides, shows,
-moves, or destroys a pane — it only cycles the tab's `swap_tiled_layout`
-states:
+That one-time retrofit *does* materialize the rail as a new sibling pane
+(see "Pane-count exception" below). Every toggle after a tab carries the
+swap set — including a dirty-tab regenerate — never creates, hides, shows,
+moves, or destroys a pane; it only cycles (or rebuilds and re-seats) the
+tab's `swap_tiled_layout` states:
```

...and immediately after the docked/undocked bullet list (before the
`hide_self` / `show_self` paragraph), add:

```markdown
**Pane-count exception (first toggle only, verified live 2026-07-08).** The
invariant above describes the toggle once a tab's swap set exists.
Installing that set the first time (`retrofit`, and the identical machinery
`regenerate_swaps` reuses for a dirty tab) runs a full-tab `override_layout`
whose base wraps the tab's existing arrangement in `pane
split_direction="vertical" { rail; region }` — the rail is a brand-new
sibling pane that did not exist before (`split_preserving_layout_kdl`,
`src/main.rs:1512-1517`). A tab with N panes before its first dock has N
panes + 1 rail after. This is accepted, not a bug: no mechanism validated
in this document reserves a tiled column on a live tab without
materializing a pane for it. A **dirty-tab regenerate** re-seats the same
rail pane by identity (`retain_existing_plugin_panes`) — pane count does
not change on a regenerate; only split-nesting fidelity can degrade (the
fidelity ceiling below, SPEC landmine #35).
```

**`SPEC.md` landmine #16** (`### Focus`, currently ending "...so nothing is
shown, hidden, or spawned."):

```diff
 the adopted architecture keeps a sidebar pane in every tab's layout and
-toggles by cycling swap layouts (`docs/docking-approach.md`), so nothing is
-shown, hidden, or spawned.
+toggles by cycling swap layouts (`docs/docking-approach.md`) once that
+tab's swap set exists; the one-time retrofit that installs it is the sole
+exception and does spawn the rail pane (`docs/docking-approach.md`'s
+pane-count exception).
```

**`SPEC.md` "If building v2 from scratch" item 3** (currently ending
"...Nothing is spawned, hidden, or shown."):

```diff
-   arrangement (`docs/docking-approach.md`, Adopted architecture). Nothing
-   is spawned, hidden, or shown. Accept a fidelity ceiling: ...
+   arrangement (`docs/docking-approach.md`, Adopted architecture). Nothing
+   already carrying the swap set is spawned, hidden, or shown — the
+   one-time retrofit is the sole exception (it spawns the rail). Accept a
+   fidelity ceiling: ...
```

## Test plan

Riskiest-first: the pane-restructuring mechanism (AC-1) was already
confirmed both offline-in-spirit (the KDL-generation structure is
deterministic and inspectable without a live session) and live (disposable
session repro, 2026-07-08) during this ideation pass — no further spike
needed before implementation. The one net-new test this task should land is
the AC-1 fixture described above (assert the generated swap KDL always
nests exactly one rail pane, N-independent) — small, offline, fast, and it
pins the structural invariant the doc diff now documents. AC-2 and AC-3
need no new tests: AC-2 resolves to a doc statement (nothing to test, the
absence of a `new_tab` call is a static fact), and AC-3 is already covered
by existing tests (`is_redundant_tiled_sidebar` suite,
`src/main.rs:3405-3415`) plus the live-validated fidelity-ceiling writeup.

## Out of scope

Redesigning the swap-cycling docking architecture wholesale, or re-opening
the "runtime tiled docking is unwinnable" question `docs/docking-approach.md`
already settled — this task is about the undocumented pane-restructuring
side effect and the new-tab mystery, not the overall docking approach.

**Follow-on finding (not fixed here): floating sidebar-instance leak.**
Live inspection during this ideation pass found 62 stray floating
`zellij-sidebar.wasm` panes accumulated across the live `WORK` session's
tabs (14/"Noteplan", 47/"CEO", 1/"Tab #6") — config-matched bootstrap
instances that never got promoted to a tiled resident and never closed
themselves (`is_stray_floating_bootstrap` only fires once a *tiled* sidebar
exists in the same tab). This is a real, severe, currently-unowned bug
(unbounded pane accumulation in a long-lived session) and the most likely
explanation for the "new tab" sighting, but its trigger (candidate:
`Alt /` racing a `./build.sh` rebuild — `skip_cache true` forces a fresh
compile per launch, and the shared `zellij.log` shows ENOENT bursts for the
sidebar wasm path at points across the session, though the log is shared
across concurrently running sessions/worktrees so the correlation is not
conclusive) needs its own clean, single-session repro with tracing on.
Recommend filing a new finding entity for it rather than folding it into
this one.

## Stage Report: ideation

- DONE: Root-cause the new-tab creation observed live on Alt-/
  Refuted via static grep (no `new_tab`/`NewTab` call in the toggle path) and a live repro in a disposable `zellij` session (real `Alt /` keypress): tab count stayed at 1 while pane count went 1→2. Read-only `dump-layout` of the live `WORK` session found the likely real cause instead — 62 stray floating sidebar panes across 3 of 4 tabs.
- DONE: Decide and record the fix direction for the first-toggle pane-split
  Direction (b) — document the exception — chosen over a non-invasive rewrite, because docs/docking-approach.md already ruled out every non-invasive alternative (floating overlay, hide/show, blind absorb); recorded under "Fix direction" with citations.
- DONE: Propose the doc diff for docs/docking-approach.md and SPEC.md
  Diffs for docking-approach.md:266-270 plus a new exception paragraph, SPEC.md landmine #16, and SPEC.md's "If building v2 from scratch" item 3 (a third over-broad-invariant site the original finder didn't cite) are in "Proposed doc diff" above.

### Summary

Both ideation questions are resolved by direct investigation rather than further design: the pane-split is inherent to the only validated dock mechanism (override_layout retrofit) and should be documented, not re-engineered; the "new tab" sighting is refuted as an actual tab and most likely explained by a separate, serious floating-instance leak found live in CL's WORK session (62 stray panes), which is flagged as its own follow-on finding rather than fixed here. AC-1/AC-2/AC-3 are rewritten to verifiable, non-tautological checks, and the concrete doc diff is ready for review at this gate.
