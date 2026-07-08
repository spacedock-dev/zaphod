---
id: j5zfk59gesvg0zfyw23haf56
title: Dock toggle restructures panes on first activation — contradicts docking-approach.md invariant
status: validation
source: finding — CL live session, Alt-/ dock regression, 2026-07-08
started: 2026-07-08T07:03:01Z
completed:
verdict:
score: 0.8
worktree: .worktrees/spacedock-ensign-dock-toggle-restructures-panes
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

### New live finding (mid-ideation, from team-lead): chrome misplacement on a dirty-tab regenerate — OPEN, not yet reproduced

Team-lead captured a second live dump of `WORK`'s tab "Tab #6" after this
report's first pass: rail docked already, then CL closed one of the tab's
other panes (a manual removal — dirtying the swap layout, same mechanism as
AC-3), then toggled. The result: `zellij:tab-bar` is missing from its
canonical `pane size=1 borderless=true` top row and instead appears as a
`pane size="50%" borderless=true { plugin location="zellij:tab-bar" }`
**sibling inside the vertical split**, next to the surviving `command=
"spacedock"` pane (also `size="50%"`) — the tab bar visibly breaks
(cramped to 40% width per CL, nothing below it). `status-bar` stayed
correctly placed. This is a more severe failure mode than the plain
pane-count change (AC-1): chrome, not just region nesting, ends up wrong.

**Investigation this pass, before concluding:**

- **Static trace.** Walked `extract_chrome_panes` (`src/main.rs:1613-1667`)
  against the reported shape: a 3-line `pane size="50%" borderless=true {
  plugin location="zellij:tab-bar" }` sitting as a direct child of the
  top-level vertical-split block is exactly the multi-line case the
  function's `children.iter().find_map(plugin_location)` branch is built to
  catch (not the single-line-inline case `bce89a4` fixed, and not a case
  `830f7b1`/`b5eeb97` left unhandled by inspection) — classification would
  return `PluginRole::Chrome` and drop it, if this pane really is a direct
  child of that block at the point the dump is read. No obvious
  classification bug found for the shape as reported.
- **Live repro attempts (disposable `ztestrepro` session, 2026-07-08),
  none reproduced it:** (1) dock 1 pane → add 2 more (3 total) → toggle
  (regenerate) → clean; (2) toggle back to docked → close 1 of 3 (down to
  2) → toggle → clean; (3) close a 2nd (down to 1) → toggle → clean; (4)
  replace a plain-shell pane with a `command=`/`start_suspended true` pane
  (matching CL's `spacedock` pane's shape) → close the other pane, leaving
  only the command pane → toggle twice → clean both times. All four
  variants toggled correctly with chrome staying in place. The exact
  trigger is narrower than "dock, add/remove panes, toggle."
- **Working hypothesis, not confirmed: concurrency.** My disposable session
  only ever had one sidebar instance. `WORK`'s "Tab #6" sits in a session
  that (per this same report's AC-2 finding) is carrying dozens of stray
  floating zombie instances in *other* tabs, and the relaxed election lets
  any instance that perceives a sidebar-less or dirty active tab act on it
  (`docs/docking-approach.md`'s "Tab ids vs positions" section already
  documents a stale `tab_id → position` translation race under load). A
  zombie elsewhere racing a regenerate on Tab #6 — each building its
  transform from a dump taken at a slightly different instant, then both
  calling `override_layout` on the same tab — is a plausible way to
  corrupt a layout that neither actor's transform alone would produce, and
  would explain why a clean single-instance session can't reproduce it.
  This is not verified; it is the leading lead for a follow-up spike, and
  it ties this finding to the floating-leak finding above as possibly one
  root cause surfacing two symptoms, not two unrelated bugs.

This is not resolved to the standard AC-1/AC-2/AC-3 were: it is confirmed
real (direct live evidence, not a hypothesis) but not yet reproducible on
demand, so no fix — doc-only or otherwise — can be designed for it yet. See
AC-4 and Out of scope.

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
a regenerate; the only *accepted* degradation is documented nesting-fidelity
loss, not chrome relocation.**
Verified by: existing coverage — `install_split_preserving_swaps`'s dump
abort (`src/main.rs:912-919`) plus `retain_existing_plugin_panes=true`
already re-seat the resident rail by identity rather than duplicating it on
a dirty-tab regenerate, and the pane-count/nesting-flattening degradation
mode is already live-validated in `docs/docking-approach.md`'s
fidelity-ceiling section and SPEC landmine #35 — this part needs a
citation, not a new mechanism. **Caveat added this pass:** AC-4 (below)
found a dirty-tab regenerate can also relocate a *chrome* pane, which is
not part of the accepted ceiling. AC-3 is satisfied for pane count and
region nesting; it is not yet fully satisfied until AC-4 is resolved,
since both are the same `regenerate_swaps` code path.

**AC-4 — A dirty-tab regenerate never relocates a chrome pane
(tab-bar/status-bar) out of its canonical top/bottom row into the content
region. OPEN — confirmed real, not yet reproducible on demand.**
Verified by (interactive, live, 2026-07-08): CL's `WORK` session, tab
"Tab #6" — `dump-layout` after "rail docked, close a pane, toggle" shows
`zellij:tab-bar` as a `size="50%"` sibling inside the vertical split
instead of its canonical top row; `status-bar` unaffected (see Proposed
approach). Attempted (offline-equivalent, live disposable session,
2026-07-08): four repro variants — plain 3-pane dock/close/toggle,
close-to-2, close-to-1, and a `command=`/`start_suspended` pane matching
CL's `spacedock` pane's shape — all toggled cleanly, none reproduced the
relocation. Static trace of `extract_chrome_panes` did not find an
obvious classification bug for the reported shape. Not satisfied yet: this
AC needs a repro that actually triggers the defect (leading hypothesis:
a concurrent regenerate/retrofit race from one of the leaked floating
instances found under AC-2 — see Out of scope) before a fix — doc-only or
code — can be designed. The doc diff below covers AC-1/AC-2/AC-3 only; it
does not close AC-4.

## Proposed doc diff

This diff closes AC-1, AC-2, and AC-3. It does **not** close AC-4 (the
chrome-misplacement finding above is not yet reproducible on demand, so no
doc statement or code fix can be written for it responsibly yet) — a
follow-up pass must either extend this diff with AC-4's resolution or file
it as its own finding once root-caused.

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

Riskiest-first, revised: **AC-4's chrome-misplacement repro is now the
riskiest open item** — riskier than anything else in this entity, since
it's the one confirmed-real defect this pass could not pin down, and it
may invalidate the "doc-only" fix direction if it turns out to be a code
defect rather than a zellij-side quirk. Before any implementation work: run
a **concurrency repro** — two-plus sidebar instances alive at once (one
tiled resident in the target tab, one or more floating elsewhere, as
`WORK` actually has), close a pane in the tiled tab, toggle, and check
whether a race between instances reproduces the tab-bar relocation that a
clean single-instance session (four variants tried, 2026-07-08) could not.
If that reproduces it, the fix is almost certainly a dedup/locking
concern shared with the AC-2 floating-leak follow-on, not a
`extract_chrome_panes` classification bug (static trace found none for the
reported shape). If it still does not reproduce, the next lead is
`debug "1"` tracing on `WORK`'s actual next occurrence, since the exact
live sequence may carry a detail (timing, a third instance, a specific
pane count) not yet captured.

The pane-restructuring mechanism (AC-1) was already confirmed both
offline-in-spirit (the KDL-generation structure is deterministic and
inspectable without a live session) and live (disposable session repro,
2026-07-08) during this ideation pass — no further spike needed before
implementing that part. The one net-new test this task should land for
AC-1 is the fixture described above (assert the generated swap KDL always
nests exactly one rail pane, N-independent) — small, offline, fast, and it
pins the structural invariant the doc diff now documents. AC-2 and AC-3
need no new tests: AC-2 resolves to a doc statement (nothing to test, the
absence of a `new_tab` call is a static fact), and AC-3 (pane count/nesting
only) is already covered by existing tests (`is_redundant_tiled_sidebar`
suite, `src/main.rs:3405-3415`) plus the live-validated fidelity-ceiling
writeup.

## Out of scope

Redesigning the swap-cycling docking architecture wholesale, or re-opening
the "runtime tiled docking is unwinnable" question `docs/docking-approach.md`
already settled — this task is about the undocumented pane-restructuring
side effect and the new-tab mystery, not the overall docking approach.
**AC-4 (chrome misplacement) is explicitly not out of scope** — it was
folded into this entity's ACs at team-lead's request because it shares the
same `regenerate_swaps` code path as AC-3 — but it is not yet resolved
(see AC-4, Test plan).

**Follow-on finding (not fixed here, likely one root cause behind two
symptoms): floating sidebar-instance leak, possibly the same concurrency
condition behind AC-4.**
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
Working hypothesis added this pass: the same population of leaked
instances is the leading (unconfirmed) explanation for AC-4's chrome
relocation too — a zombie racing a legitimate regenerate. If a follow-up
spike confirms that link, the leak and AC-4 should be root-caused and fixed
together rather than as two separate findings; recommend filing one new
finding entity for the instance-lifecycle/concurrency problem rather than
folding either into this one, since this entity's own fix (the doc diff)
is ready to ship independently of that investigation.

## Stage Report: ideation

- DONE: Root-cause the new-tab creation observed live on Alt-/
  Refuted via static grep (no `new_tab`/`NewTab` call in the toggle path) and a live repro in a disposable `zellij` session (real `Alt /` keypress): tab count stayed at 1 while pane count went 1→2. Read-only `dump-layout` of the live `WORK` session found the likely real cause instead — 62 stray floating sidebar panes across 3 of 4 tabs.
- DONE: Decide and record the fix direction for the first-toggle pane-split
  Direction (b) — document the exception — chosen over a non-invasive rewrite, because docs/docking-approach.md already ruled out every non-invasive alternative (floating overlay, hide/show, blind absorb); recorded under "Fix direction" with citations.
- DONE: Propose the doc diff for docs/docking-approach.md and SPEC.md
  Diffs for docking-approach.md:266-270 plus a new exception paragraph, SPEC.md landmine #16, and SPEC.md's "If building v2 from scratch" item 3 (a third over-broad-invariant site the original finder didn't cite) are in "Proposed doc diff" above. Covers AC-1/AC-2/AC-3 only, not AC-4.
- FAILED: Reproduce team-lead's live chrome-misplacement finding (tab-bar relocated into the region on a dirty-tab regenerate after a manual pane close) in a controlled disposable session
  Folded into the entity as AC-4 and a new "Root cause" subsection; four repro variants on 2026-07-08 (plain 3-pane close-down, command-pane variant) all toggled cleanly with chrome correctly placed, and a static trace of `extract_chrome_panes` found no obvious classification bug for the reported shape. Leading unconfirmed hypothesis recorded: a concurrent regenerate/retrofit race from one of the AC-2 leaked floating instances. AC-4 is open; Test plan names the concurrency repro as the next riskiest step.

### Summary

Two of three original ideation questions are resolved by direct investigation rather than further design: the pane-split is inherent to the only validated dock mechanism (override_layout retrofit) and should be documented, not re-engineered; the "new tab" sighting is refuted as an actual tab and most likely explained by a separate, serious floating-instance leak found live in CL's WORK session (62 stray panes). A new live finding arrived mid-pass from team-lead — a dirty-tab regenerate relocating the tab-bar pane into the content region — which was folded in as AC-4 but could not be reproduced in a clean session in the time available, so it remains open pending a concurrency-focused repro. AC-1/AC-2/AC-3 are rewritten to verifiable, non-tautological checks and the concrete doc diff for them is ready for review at this gate; AC-4 is not ready and should not block shipping the AC-1/AC-2/AC-3 doc diff, but should stay open as a tracked item (possibly merged with the floating-leak follow-on) rather than closed out with this entity.

## Stage Report: implementation

- DONE: Apply the proposed doc diff to docs/docking-approach.md:266-270 (the pane-count exception paragraph) exactly as drafted in the ideation body.
  Applied verbatim: replaced the unconditional invariant sentence and inserted the "Pane-count exception" paragraph after the docked/undocked bullets, before the `hide_self`/`show_self` line. Diff matches the drafted text character-for-character (verified with `git diff`).
- DONE: Apply the proposed doc diff to SPEC.md landmine #16 and the "If building v2 from scratch" item 3, exactly as drafted in the ideation body.
  Both sites replaced verbatim per the drafted diff. Cross-checked citations: `src/main.rs:1512-1517` matches the `tab_kdl` closure building `pane split_direction="vertical" { rail; main }` inside `split_preserving_layout_kdl`; SPEC landmine #35 / fidelity-ceiling citation confirmed present at `docs/docking-approach.md:674-702` and `SPEC.md:366`.
- DONE: Confirm no code changes are needed (AC-1/AC-2/AC-3 are doc-only per the ideation's own conclusion) and record that confirmation in the stage report rather than silently assuming it.
  Confirmed: only `docs/docking-approach.md` and `SPEC.md` were touched, no `src/` changes. Ran `cargo check --tests` (clean, `Finished dev profile` in 1m08s) and `cargo test` (130 passed; 0 failed; 0 ignored) on the worktree after the doc edit — suite is unaffected, consistent with the ideation's conclusion that AC-1/AC-2/AC-3 need no new test beyond what already exists (the AC-1 KDL fixture is named in Test plan as future work, not part of this doc-only stage). AC-4 remains explicitly out of this stage's scope (open, per entity body).

### Summary

Applied both doc diffs from the ideation body verbatim to `docs/docking-approach.md` (pane-count exception paragraph) and `SPEC.md` (landmine #16, "If building v2 from scratch" item 3), committed to the worktree branch as `baff70e`. Verified the diffs' code citations still hold and ran the full native test suite (130/130 passing, no code changes) to confirm this stage is doc-only as the ideation concluded. AC-4 stays open and untouched, as scoped.

## Stage Report: validation

- DONE: Independently re-run the doc-diff verification: diff docs/docking-approach.md and SPEC.md against the ideation body's drafted text, confirm character-for-character match, not a re-read of the implementer's claim.
  Checked out `baff70e` (the worktree's committed HEAD) in a separate throwaway worktree and byte-diffed each drafted hunk against the live file text at the cited line ranges: docking-approach.md:266-272 (invariant-sentence replacement) and :278-291 (new "Pane-count exception" paragraph) both `diff`-clean against the drafted text; SPEC.md:158-161 (landmine #16) and :389-392 (v2-from-scratch item 3, verified up to the "..." elision point, confirmed via `git show baff70e -- SPEC.md` that the elided trailing text is genuinely unchanged) both `diff`-clean. All four sites match character-for-character.
- DONE: Run a refutation audit on a throwaway checkout (never the implementation worktree): re-verify the code citations actually say what the doc now claims, and look for any other stale invariant claim elsewhere in the two files this diff should have caught but didn't.
  Used the same throwaway detached worktree (`git worktree add --detach`, removed after). `split_preserving_layout_kdl`'s `tab_kdl` closure at src/main.rs:1512-1517 confirmed: builds `pane split_direction="vertical" { rail; main }` wrapping the dumped region — matches the doc's claim exactly. SPEC landmine #35 (SPEC.md:346-367) and docking-approach.md's fidelity-ceiling section (:674-702) both independently confirm "panes and content survive, only split nesting is lost" — matches the exception paragraph's citation. Traced `install_split_preserving_swaps` (src/main.rs:889-940): confirmed both `retrofit` and `regenerate_swaps` call the identical function (matches "identical machinery... reuses for a dirty tab"), and confirmed the `override_layout` call's `retain_existing_plugin_panes=true` / `apply_only_to_active_tab=true` args (src/main.rs:934-939) match the doc's re-seat-by-identity claim and the entity's own new-tab refutation.
  REFUTED-elsewhere finding: `README.md:34-37` states the toggle "cycl[es] the tab's swap layouts — panes are rearranged in place, never spawned or hidden" — the exact unconditional claim this diff just corrected in the other two files, left uncorrected in a third. `git log -S "never spawned or hidden" -- README.md` traces it to commit `08ffe98` ("Preserve manual splits across toggles by regenerating swaps," 2026-07-03), one commit after the regression-introducing `3154259` (2026-07-02) — this line has been false since the day after the regression landed, not a preexisting quirk. Not one of "the two files" the checklist names, but it's the same stale invariant this task exists to fix, in a doc a user reads first (README feature list). Flagging for the FO to either fold into this entity's doc diff or file as a fast-follow before closing this entity — leaving it would mean a shipped "done" fix still ships a false claim about the same behavior one file over.
- DONE: Confirm AC-4 (chrome misplacement) is correctly left open/out of scope for this entity and not silently treated as resolved by this doc-only stage.
  Confirmed via `git show baff70e` full diff: zero mentions of `extract_chrome_panes`, `tab-bar`, `status-bar`, or chrome-relocation language in the actual changed lines (the two "chrome" hits are pre-existing unchanged context lines). Entity body's AC-4 section, "Proposed doc diff" preamble, and "Out of scope" section all still explicitly state AC-4 is open/unresolved; frontmatter `status: validation` / `completed:` / `verdict:` are all unset — nothing marks this entity done. AC-4 is correctly untouched by this stage.

### Summary

Re-ran the doc-diff verification independently on a throwaway detached checkout at the worktree's exact commit (`baff70e`) rather than trusting the implementer's report: all four drafted hunks across `docs/docking-approach.md` and `SPEC.md` match the shipped text character-for-character, and every code/SPEC citation in the new text checks out against the actual source. The refutation audit surfaced one real gap outside the checklist's named scope: `README.md` carries the identical false "never spawned or hidden" toggle claim, introduced by the same regression window (`08ffe98`, the commit right after the regression landed) and untouched by this doc-only fix — recommend the FO decide whether to fold it into this entity or fast-follow it before treating the invariant as fully corrected. AC-4 remains correctly open and unresolved; this stage did not touch or silently close it.
