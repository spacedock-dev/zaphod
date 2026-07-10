---
id: j5zfk59gesvg0zfyw23haf56
title: Dock toggle restructures panes on first activation — contradicts docking-approach.md invariant
status: ideation
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

j5 cannot restore exact first-toggle terminal preservation through Zellij
0.44.3's public plugin API. The two candidate families fail at the host
boundary: retained panes are appended without a transaction, and plugins
cannot read a terminal's original typed `invoked_with()` value.

The exact server path explains the destructive failure. In tag `v0.44.3`
(commit `55a2121`), `LayoutApplier::override_tiled_panes_layout_for_existing_panes`
drains the tab, places exact-run matches and new panes, then passes each
remaining retained pane to `handle_remaining_tiled_pane_ids`. That function
removes each pane from `ExistingTabState` before calling
`TiledPanes::insert_pane`. If `add_pane_without_stacked_resize` cannot split a
pane or add to a stack, it logs `Failed to add pane to stack` and returns
without reinserting the owned pane. No preflight, rollback, or failure result
restores the drained pane. `Tab::override_layout` also installs the new base
and swaps before it runs this mutation, and the plugin shim returns `()`.

KDL offers no non-spawning variadic retained-pane slot. `children` is a
parse-time template placeholder. The parser replaces an unresolved placeholder
with one `TiledPaneLayout::default()` leaf; the override path calls
`flatten_layout(..., false)`, so it never expands that leaf to the number of
existing panes. A disposable two-terminal explicit-cwd session confirmed the
server behavior: a `stacked { children }` base retained terminal IDs 0 and 1
but spawned terminal ID 2 for its single bare `None` slot. A chrome plus
flexible-plugin-anchor base happened to retain IDs 0 and 1 at 80x24, but it
uses the same one-by-one `insert_pane` path and the same pane-dropping error
branch. A happy geometry cannot make that mechanism transactional.

The identity fallback is also unavailable. `PaneUpdate` exposes only
`PaneInfo::terminal_command`, a flattened display string for `Run::Command`;
bare `None` and `Run::Cwd` both appear as `None`, and cwd, argument boundaries,
`hold_on_close`, `hold_on_start`, and other run variants are absent.
`get_pane_cwd` and `get_pane_running_command` explicitly query current OS
state. `dump_session_layout_for_tab` starts from `invoked_with()`, but
`Pty::populate_session_layout_metadata` overwrites terminal commands and cwds
from current processes before serialization. Three disposable controls
falsified those substitutes:

| Original identity | Public/current substitute used in a concrete retained base | Result |
|---|---|---|
| bare `None`, shell currently in the worktree | `cwd=<worktree>` | terminal IDs `{0}` became `{0,1}` |
| `Run::Cwd("/tmp")`, shell later `cd` to the repo | `cwd=<repo>` | terminal IDs `{0}` became `{0,1}` |
| `Run::Command("/bin/sh", ["-c", "sleep 600"], cwd=/tmp)`, current child `sleep 600` | `command="/bin/sleep"`, args `"600"`, cwd `/tmp` | terminal IDs `{0}` became `{0,1}` |

The exact matcher in `screen.rs::find_already_running_panes` requires the
original typed value. None of these surfaces can round-trip it.

## Proposed approach

Stop repository implementation and present a captain gate. The current host
supports neither safe mechanism that j5 requires.

### Choice A — patch or upstream Zellij (recommended)

Add one server-owned transactional retained-pane operation. The operation must
bind the complete retained pane-ID set to non-spawning base positions, validate
all base geometry before draining live panes, and either commit every pane plus
the rail or leave the original tab untouched. It must report success or failure
to the plugin before any swap steering. A variadic retained-children marker is
one possible KDL surface; a pane-ID-to-position API is another. The contract,
not its syntax, is decisive: no pane may leave server ownership until every
retained pane has a valid target.

This option keeps the runtime retrofit and avoids serializing launch identity.
It is the smallest architecture that can satisfy both duplication and loss
invariants, but it expands scope into a Zellij fork or upstream change.

### Choice B — reopen the retrofit architecture

Remove full-tab retained override from first-toggle docking. Tabs born from the
Zaphod layout may continue to use preinstalled swaps. Foreign tabs must use a
mechanism that does not rewrite the live tiled layout, such as a floating rail,
or require the user to open a layout-born tab before tiled docking. This option
avoids a host fork but changes the first-toggle experience and the shipped
"dock any tab" promise.

Adding only a typed `get_pane_invoked_with` API is rejected as the primary
repair. It would still require exact support for every `Run` variant and future
metadata, and the nontransactional override would still drop a retained pane
when geometry cannot seat it. Post-override cleanup is also rejected: it cannot
recover a dropped process and cannot distinguish a concurrent user-created
terminal from a spawned placeholder.

## Acceptance criteria

### Offline

**AC-1 — First activation preserves the complete terminal-ID set.** For N=1,
N=2 stacked, and N=3 mixed-split disposable tabs, the terminal IDs after one
activation equal the IDs captured immediately before it. Exactly one rail is
present; no terminal is added, removed, replaced, exited, or suppressed.

Verified by: a process-level comparison of `list-panes -a --json` ID sets. The
independent red baselines are the explicit-cwd/bare/current-command duplicate
controls and the clean N=2 no-leaf loss.

**AC-2 — Failure is atomic.** Force the base or immediate swap to hit
unsatisfiable geometry with two retained terminals. The operation reports
failure and leaves the original pane IDs, processes, geometry, and swap set
usable; it performs no partial install.

Verified by: a Zellij server integration test that snapshots pane IDs and
geometries before the call, injects the constraint failure, and compares the
complete state afterward, plus disposable server logs and pane listings.

**AC-3 — Base safety does not depend on a later swap.** Before any swap is
applied, every retained terminal and the rail occupy valid base positions. A
failed or skipped swap cannot alter terminal survival.

Verified by: a server test that pauses after base commit, inspects all pane IDs
and non-overlapping geometries, then independently fails swap application.

**AC-4 — Launch-identity variants remain stable.** Bare `None`, explicit cwd,
launch-cwd-A/current-cwd-B, mixed launch cwd, and command panes with distinct
args and hold metadata all preserve their original IDs.

Verified by: the same process matrix. Choice A must pass without reading launch
identity. Any future identity-based alternative must expose and round-trip the
typed original `Run` value rather than current process state or display text.

**AC-5 — The selected architecture closes the host gap.** Choice A exposes a
transactional result-bearing host operation; Choice B performs no retained
full-tab override on foreign tabs and documents the replacement experience.

Verified by: API-level tests for Choice A or an action trace for Choice B that
proves no `OverrideLayout` command is emitted during foreign-tab activation.

**AC-6 — Documentation follows proved behavior.** README, SPEC, and
`docs/docking-approach.md` claim exact terminal preservation only after AC-1
through AC-5 pass. Until then, the unreliable-count caveat and this host-API
blocker remain explicit.

Verified by: review of the conditional diff below against the process matrix
and atomic failure test.

### Interactive

**AC-7 — CL sees no terminal loss or duplicate in the multi-pane case.** In a
disposable two-terminal stacked session, CL presses `Alt /` once and sees the
same two shells beside one rail; their before/after IDs match. Repeat with the
launch-cwd-A/current-cwd-B case. Do not use `WORK` unless CL separately
authorizes it.

Verified by: the live demo paired with pane-ID listings, not visual inspection
alone.

## Proposed doc diff

No production doc should present a replacement mechanism until the captain
chooses an architecture and AC-1 through AC-5 pass. Keep the current caveat.
After Choice A passes, apply this semantic diff:

**`docs/docking-approach.md` — replace the current unreliable-count exception:**

```diff
-**Pane-count exception (first toggle only; exact resulting count not yet a
-reliable invariant).** ...
+**First-toggle terminal preservation.** Installing the swap set materializes
+exactly one rail and preserves the complete set of existing terminal pane IDs.
+Zellij seats retained panes through a transactional, non-spawning host
+operation before it installs or applies swaps. Terminal survival does not
+depend on the immediate swap relayout: rejection leaves the original tab and
+every terminal unchanged.
```

Update the Toggle v3 mechanism section with the proved base construction and
its N>=2 failure behavior. Do not name `stacked { children }`, a no-leaf base,
or a concrete identity base as canonical unless that exact construction passed
the full matrix.

**`SPEC.md` landmine #26 — record both rejected layout-only assumptions:**

```diff
-An override KDL must absorb existing panes via `pane stacked=true
-{ children }`.
+Retained override safety has two independent hazards. A spawnable slot whose
+`Run` differs from a pane's original `invoked_with()` duplicates that pane; a
+base with no terminal seating geometry can drop N>=2 retained panes before an
+unsatisfiable swap relayout. A valid retrofit must preserve the complete pane-
+ID set in the base itself and remain non-destructive when relayout fails.
```

Add a SPEC landmine recording that Zellij 0.44.3's retained override drains and
reinserts unmatched panes without rollback, and that its plugin surfaces do not
expose original typed `invoked_with`. After Choice A passes, align README, SPEC
landmine #16, and “If building v2 from scratch” item 3 to say that first toggle
adds only the rail and preserves terminal IDs. Choice B instead requires a
captain-reviewed doc diff for its changed foreign-tab experience.

## Test plan

1. **Smallest invalidation first.** Against unpatched Zellij 0.44.3, create a
   two-terminal tab, force the retained base to run out of insertion geometry,
   and assert that a result-bearing call rejects without changing pane IDs or
   swaps. The current server fails this test because it drains panes, reinserts
   them one by one, and has no rollback or result channel.
2. For Choice A, add server unit tests for preflight and rollback before the
   plugin integration. Prove that all retained pane-ID targets are valid before
   any drain, spawn, close, or swap-set mutation.
3. Run the full process matrix: N=1/N=2/N=3; bare, explicit cwd, changed cwd,
   mixed cwd, command plus distinct args/hold metadata; stacked and split
   geometries; canonical and misplaced chrome. Each case activates once and
   compares terminal ID sets.
4. Inject unsatisfiable base and swap layouts. Confirm an atomic rejection and
   unchanged original state, then run successful docked/undocked cycles and
   confirm the ID set remains unchanged.
5. For Choice B, replace steps 2 through 4 with an action trace proving foreign
   tabs never call retained `override_layout`, then test and document the
   captain-approved replacement interaction.
6. Only after the selected architecture passes its process tests, update the
   repository transform and docs, run the full native suite, and perform AC-7's
   disposable interactive demo.

## Out of scope

- Implementing the refuted no-terminal-leaf design or treating its N=1 result
  as sufficient evidence.
- Post-hoc closing of newly observed terminal IDs, recovery after a terminal
  process has already been removed, or any heuristic based on title/current cwd.
- eh's floating-instance leak, multi-actor serialization, and chrome placement.
  The clean N=2 reproduction keeps those independent of j5.
- Shipping a Zellij fork or reopening docking architecture without a new
  captain gate. This ideation supplies that gate; it does not choose for CL.
- Any mutation of `WORK`; it supplies read-only evidence only.

## Refuted cycle-3 no-terminal-leaf design

The following cycle-3 body is retained for audit. Its single-terminal root
cause remains valid, but its proposed no-leaf repair, ACs, and doc diff are
superseded by the N>=2 terminal-loss reproduction above.

### Problem (cycle 3, refuted)

The first dock toggle must add the rail without duplicating a user's terminal.
The current runtime retrofit violates that invariant when the existing terminal
has an explicit launch identity. In `WORK`, one terminal became two terminals
plus the rail after one toggle. This is deterministic terminal duplication, not
an accepted cost of reserving a tiled rail column.

The mismatch crosses three layers:

1. `WORK`'s live serialized template at
   `/Users/clkao/Library/Caches/org.Zellij-Contributors.Zellij/contract_version_1/session_info/WORK/session-layout.kdl`
   contains `new_tab_template { pane cwd="/Users/clkao" }`. The live terminal's
   `invoked_with()` value is therefore `Some(Run::Cwd("/Users/clkao"))`.
2. `split_preserving_layout_kdl` emits the override base as
   `pane stacked=true { children }`. Zellij extracts a bare terminal slot from
   that base with launch identity `None`.
3. Zellij 0.44.3's server-side `find_already_running_panes` compares each
   layout run instruction with each pane's `invoked_with()` by exact equality.
   `None` does not match `Some(Run::Cwd(...))`, so the server spawns a terminal
   for the bare base slot. `retain_existing_terminal_panes=true` independently
   keeps the original terminal. The result is the original terminal, a new
   default terminal, and the rail.

The prior clean-room result did not reproduce `WORK`'s launch identity. A bare
`pane` starts a terminal with identity `None`, which matches the current base's
`None` slot; that control stays at one terminal plus the rail. A one-call CLI
reproduction with an explicit-cwd terminal and a builtin compact-bar rail
produced the duplicate without the Zaphod plugin, sidebar population, a second
override, or concurrent actors. The earlier TOCTOU/shared-race theory is not
the cause of this symptom.

### Proposed approach (cycle 3, refuted)

Change only the runtime override base generated by
`split_preserving_layout_kdl`: emit canonical chrome and the rail slot, but no
spawnable terminal leaf and no `children` main. Keep the dumped terminal
arrangement in the docked and undocked swap layouts. With
`retain_existing_terminal_panes=true`, Zellij appends the real terminals while
applying the base; its immediate swap relayout then re-seats those terminals
into the dumped arrangement. The base no longer invents a launch identity that
must match every retained terminal.

This is the smallest change because it preserves the existing dump transform,
rail identity, chrome extraction, retain flags, swap-state steering, and
active-tab scope. The pure function extended is
`split_preserving_layout_kdl`; `tab_kdl` should support a rail-only base while
continuing to receive `region_kdl` for both swap layouts. No production code is
changed during this ideation stage.

#### Smallest mechanism spike — N=1 only, refuted for N>=2

The riskiest assumption was that Zellij would accept a base with no terminal
leaf, retain the real terminal as an extra, and then re-seat it through the
installed swap. Four disposable Zellij 0.44.3 sessions exercised one active-tab
CLI `override-layout` call each, with both retain flags. The rail used builtin
`zellij:compact-bar`, so the Zaphod plugin and its population path were absent.

| Initial terminal identity | Override base | Result after one call |
|---|---|---|
| explicit `cwd="/Users/clkao"` | current `stacked { children }` | `terminal_0 + terminal_1 + rail` |
| bare `pane` | current `stacked { children }` | `terminal_0 + rail` |
| explicit `cwd="/Users/clkao"` | chrome + rail, no terminal leaf | `terminal_0 + rail`; cwd preserved |
| bare `pane` | chrome + rail, no terminal leaf | `terminal_0 + rail` |

The no-leaf base passed the invalidation spike and preserved the bare-pane
control. Each successful result dumped as the 28-column rail beside the single
retained terminal, which shows that the installed swap re-seated the terminal;
the base itself had no such terminal slot. All sessions were disposable and
were torn down. `WORK` was never mutated.

#### Alternatives considered (cycle 3)

- Mirror each terminal's exact `Run` identity in the base. Rejected: the dump
  is resurrection KDL, arbitrary commands and cwd variants expand the matching
  surface, and any imperfect identity recreates the duplicate.
- Disable `retain_existing_terminal_panes`. Rejected: it replaces user
  terminals and loses their live processes, the opposite of the invariant.
- Change Zellij's exact matcher or add fuzzy matching. Rejected: this project
  need not patch the server when a valid no-leaf layout construction avoids the
  mismatch.
- Serialize retrofit actors. That remains relevant to eh's floating-instance
  and chrome investigation, but the one-call reproduction proves it cannot fix
  this deterministic duplication by itself.

### Acceptance criteria (cycle 3, refuted)

#### Offline (cycle 3)

**AC-1 — A retained explicit-cwd terminal is not duplicated by first-toggle
retrofit.** Given a disposable Zellij 0.44.3 session whose only user terminal
is `terminal_0` launched from `pane cwd="/Users/clkao"`, exactly one
active-tab `override-layout` call using the production-generated layout and
both retain flags leaves exactly `terminal_0` plus exactly one rail; no second
terminal ID exists. Chrome remains in its canonical rows.

Verified by: an agent-reproducible process-level regression that records
`list-panes -a` before and after the single call and compares terminal IDs and
counts. The independent failure baseline is the current implementation's live
result, `terminal_0 + terminal_1 + rail`, reproduced with the same explicit-cwd
fixture and one call.

**AC-2 — The generated base has no spawnable terminal leaf while both swaps
retain the dumped terminal arrangement.** For explicit-cwd, bare-pane, and
multi-terminal dump fixtures, the base contains canonical chrome plus one rail
slot and zero terminal slots; the docked and undocked swap tabs each contain
the transformed region and one rail slot.

Verified by: focused tests extending `split_preserving_layout_kdl` and its
existing KDL block helpers to inspect base and swap structure, followed by the
process-level AC-1 proof. A string or substring assertion alone cannot satisfy
this AC.

**AC-3 — The bare-pane control remains one terminal plus one rail.** Given a
disposable session whose only user terminal comes from bare `pane`, the same
single retained override call leaves the original terminal ID plus one rail
and no extra terminal.

Verified by: the paired process-level control, using the same layout except
for the initial pane's absent cwd identity. This guards against fixing
`Some(Run::Cwd)` by breaking the existing `None` case.

**AC-4 — Documentation states the restored boundary.** README, SPEC, and
`docs/docking-approach.md` state that first retrofit materializes one rail pane
but preserves the existing terminal set; they no longer describe extra or
unreliable terminal count as accepted architecture. The exact-launch-identity
landmine and no-terminal-leaf base are recorded.

Verified by: review of the concrete diff below against AC-1's process-level
result and the Zellij matcher behavior, not by prose matching itself.

#### Interactive (cycle 3)

**AC-5 — The first toggle visibly docks without opening a second shell.** In a
disposable session created from the explicit-cwd reproduction layout, CL
presses `Alt /` once and sees the rail dock beside the same single shell; a
pane listing confirms one terminal and one rail. `WORK` remains untouched
during validation unless CL separately authorizes using it.

Verified by: CL's live demo after implementation, paired with the before/after
pane listing so visual similarity cannot hide a duplicate stack.

### Proposed doc diff (cycle 3, withdrawn)

The implementation should apply the following semantic changes together with
the code fix. Exact wrapping may follow each file's local style.

**`docs/docking-approach.md` — replace “Pane-count exception” with the restored
invariant and mechanism:**

```diff
-**Pane-count exception (first toggle only; exact resulting count not yet a
-reliable invariant).** ... Treat "the first toggle materializes at least one
-new pane" as accepted architecture ...
+**First-toggle terminal preservation.** Installing a tab's swap set the first
+time materializes exactly one new pane: the rail. It preserves every existing
+terminal pane and terminal ID. The generated override base declares chrome and
+the rail but no terminal leaf; `retain_existing_terminal_panes` keeps the live
+terminals, and the installed docked/undocked swap re-seats them into the dumped
+arrangement. A former `stacked { children }` base duplicated terminals whose
+stored launch identity was `Run::Cwd`, because Zellij's exact matcher could not
+match that pane to the base's bare `None` slot.
```

In the Toggle v3 transform description, replace “base = dumped chrome + rail
slot + `stacked { children }` (the only retained-pane-correct override shape)”
with “base = dumped chrome + rail slot and no terminal leaf; swaps = dumped
chrome + rail + transformed terminal arrangement.” Remove the later
`children`-stub leaf-count explanation.

**`README.md` — replace the first-toggle caveat:**

```diff
-The one-time retrofit ... does spawn the rail pane, and the exact resulting
-pane count is not yet a reliable invariant ...
+The one-time retrofit materializes the rail pane while preserving the tab's
+existing terminal panes; after the first toggle, the terminal IDs and count
+are unchanged and exactly one rail is present.
```

**`SPEC.md` landmine #16 — narrow the exception to the rail:**

```diff
-the one-time retrofit that installs it is the sole exception and does spawn
-the rail pane
+the one-time retrofit materializes the rail pane but preserves the existing
+terminal set; it must not spawn a replacement or extra terminal
```

**`SPEC.md` landmine #26 — correct the retained-pane rule:**

```diff
-An override KDL must absorb existing panes via `pane stacked=true
-{ children }`.
+Retained-pane matching uses exact `Run` identity. A bare terminal slot does
+not match an existing `Run::Cwd` pane and therefore spawns a duplicate while
+the retain flag keeps the original. Runtime retrofit bases must declare no
+terminal leaf; the installed swaps re-seat the retained terminals.
```

In “If building v2 from scratch,” item 3, change “complete KDL: chrome,
stacked main, both swaps” to “complete KDL: a chrome-and-rail-only base plus
both swaps carrying the dumped arrangement,” and state that the retrofit adds
only the rail while preserving terminal IDs and count.

### Test plan (cycle 3, refuted)

1. **Riskiest mechanism first — already exercised in ideation.** Start an
   attached disposable Zellij 0.44.3 session from a layout containing exactly
   one `pane cwd="/Users/clkao"`. Issue exactly one active-tab CLI
   `override-layout` call with both retain flags and a builtin compact-bar rail.
   Current `stacked { children }` base must reproduce two terminals plus rail;
   no-leaf base must produce the original terminal plus rail. Repeat with a
   bare-pane control. These four runs passed on 2026-07-10 and were torn down.
2. Add structural unit coverage around `split_preserving_layout_kdl` for an
   explicit-cwd single terminal, a bare single terminal, and a multi-terminal
   dump. Assert through parsed/block structure that the base has no terminal
   leaf and each swap retains the transformed region and exactly one rail.
3. Turn the disposable explicit-cwd and bare-pane cases into a repeatable
   process-level regression. Capture terminal IDs before the call, wait for the
   rail to appear, and compare IDs after. Require exactly one override call;
   fail on any new terminal ID, missing chrome row, missing rail, timeout, or
   nonzero CLI exit. Kill the sessions in cleanup even on failure.
4. Run the focused Rust tests, the process-level regression, and the full native
   suite. Then perform AC-5's one-press interactive demo in a disposable
   explicit-cwd session.

### Out of scope (cycle 3)

- The floating sidebar-instance leak, lifecycle/dedup, and any concurrency
  repair belong to `dock-floating-leak-and-chrome-misplacement` (eh).
- Chrome corruption belongs to eh even when it appears in the rail slot. The
  2026-07-10 read-only `WORK` finding — plugin_84 titled `sidebar` but running
  `zellij:status-bar` at x=0, width=28, height=58, with no real Zaphod sidebar
  and no bottom status bar — is additional eh evidence, not a j5 mechanism.
- Reproducing or repairing dirty-tab tab-bar/status-bar relocation, swap-dirty
  fidelity loss, or zombie-driven multiple overrides.
- Patching Zellij's server matcher, redesigning swap-layout docking, or
  changing layout-born tabs. j5 changes only the generated runtime retrofit
  base and the regression/docs that define terminal preservation.

## Superseded ideation and validation history

The sections below preserve the earlier investigation and reports for audit.
Their clean-session evidence remains useful, but their conclusions are
superseded: the first-toggle extra terminal is not accepted architecture, and
the TOCTOU/double-override premise is not its root cause. j5 now owns the
deterministic `Run::Cwd` versus bare-slot duplication described above; eh owns
the floating leak, concurrency, and chrome symptoms.

### Problem (superseded)

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

### Proposed approach (superseded)

Both open questions are settled by direct investigation (static reading plus
a live repro in a disposable session; see evidence below), not by further
brainstorming:

#### Root cause: new-tab creation — REFUTED as a toggle-caused tab

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

#### New live finding (mid-ideation, from team-lead): chrome misplacement on a dirty-tab regenerate — OPEN, not yet reproduced

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

#### Fix direction: document the first-toggle pane-count change — chosen over a non-invasive rewrite

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

### Acceptance criteria (superseded)

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
**Caveat added Cycle 1 (2026-07-09):** the live disposable-session repro
cited above (1→2 panes, zero tabs) is not the only observed outcome. A
counter-example in the real `WORK` session (Tab #7: a freshly created tab,
N=1, a single `Alt /`) went to N=3, not N=2 — see Feedback Cycles below.
This does not contradict AC-1's *offline* claim (the generated swap KDL
template always nests exactly one rail pane as a sibling of the absorbed
region, independent of N) — that fixture test was never actually built and
remains future work per Test plan, so the offline claim is untested either
way, not disproven. It does mean the *live* claim of a clean, universal
N→N+1 count is no longer accepted as reliable. The doc diff below is
revised accordingly: AC-1 is satisfied for "a new pane is accepted
architecture," not for "the resulting count is always N+1."

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

### Proposed doc diff (superseded)

This diff closes AC-1, AC-2, and AC-3. It does **not** close AC-4 (the
chrome-misplacement finding above is not yet reproducible on demand, so no
doc statement or code fix can be written for it responsibly yet) — a
follow-up pass must either extend this diff with AC-4's resolution or file
it as its own finding once root-caused.

**Revised Cycle 1 (2026-07-09).** The `docs/docking-approach.md` block below
is no longer the text originally drafted at ideation — that text asserted a
clean, universal "N panes + 1 rail" invariant, which a live counter-example
(`WORK` Tab #7, see Feedback Cycles) disproved. The block below is what was
actually shipped this cycle: it keeps "a new pane is accepted architecture"
(still true and still evidenced) but drops the "+1" quantification and
records the counter-example plus a pointer to the open, unconfirmed
shared-root-cause investigation instead. The `SPEC.md` blocks are unchanged
from the original draft — neither site quantifies the resulting pane count,
so neither needed revision. A new `README.md` block is added this cycle
(see Feedback Cycles / validation's REFUTED-elsewhere finding).

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
`hide_self` / `show_self` paragraph), add (Cycle 1 revision — see note
above):

```markdown
**Pane-count exception (first toggle only; exact resulting count not yet a
reliable invariant).** The invariant above describes the toggle once a
tab's swap set exists. Installing that set the first time (`retrofit`, and
the identical machinery `regenerate_swaps` reuses for a dirty tab) runs a
full-tab `override_layout` whose base wraps the tab's existing arrangement
in `pane split_direction="vertical" { rail; region }` — the rail is a
brand-new sibling pane that did not exist before
(`split_preserving_layout_kdl`, `src/main.rs:1512-1517`). This is accepted,
not a bug: no mechanism validated in this document reserves a tiled column
on a live tab without materializing a pane for it. What is **not** settled
is the exact resulting count. A clean, single-instance disposable session
goes from N panes to N+1 (verified live 2026-07-08). A counter-example in a
real, long-running session instead went from N=1 to N=3 on a single first
toggle: `WORK` Tab #7, a freshly created tab, one pre-existing pane —
`list-panes -a` showed two new terminal panes (`terminal_41`, `terminal_42`)
plus the rail, and `dump-layout` confirmed three top-level siblings
(`sidebar`, two `size="50%"` panes) instead of one absorbed slot (verified
live 2026-07-09). This document does not yet know why the two cases
differ; it is suspected, unconfirmed, to share a root cause with the
concurrent floating-sidebar-instance leak tracked separately in
`dock-floating-leak-and-chrome-misplacement` (open as of this writing).
Treat "the first toggle materializes at least one new pane" as accepted
architecture; do not treat "exactly one new pane" as reliable until that
investigation closes. A **dirty-tab regenerate** re-seats the same rail
pane by identity (`retain_existing_plugin_panes`) — pane count does not
change on a regenerate; only split-nesting fidelity can degrade (the
fidelity ceiling below, SPEC landmine #35).
```

**`README.md:34-37`** — added this cycle (validation's REFUTED-elsewhere
finding): the same unconditional "never spawned or hidden" claim, in a
third file the original checklist didn't name:

```diff
 - `Alt /` (or the `⇄` header) toggles the docked 28-col rail down to a
-  1-col sliver and back by cycling the tab's swap layouts — panes are
-  rearranged in place, never spawned or hidden, and a manually re-split tab
-  keeps its arrangement (the swap set is regenerated from the live layout)
+  1-col sliver and back by cycling the tab's swap layouts once a tab
+  already carries that swap set — panes are rearranged in place, never
+  spawned or hidden, and a manually re-split tab keeps its arrangement (the
+  swap set is regenerated from the live layout). The one-time retrofit that
+  installs the swap set on a tab's first toggle is the sole exception: it
+  does spawn the rail pane, and the exact resulting pane count is not yet a
+  reliable invariant (see `docs/docking-approach.md`'s pane-count
+  exception)
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

### Test plan (superseded)

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

### Out of scope (superseded)

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

### Feedback Cycles

**Cycle 1 (2026-07-09) — REJECTED at validation, routed to implementation.**

CL live-tested the first-toggle case directly in the `WORK` session (Tab #7:
a freshly created tab, single `Alt /`) and found a case this entity's own
"accepted invariant" doesn't cover: `zellij --session WORK action list-panes
-a` shows two live `/bin/zsh` terminal panes (`terminal_41`, `terminal_42`,
both cwd `/Users/clkao`) plus the sidebar (`plugin_88`) in Tab #7 —
**N=1 → N=3, not the documented N=1 → N=2.** `dump-layout` confirms the
outer split as three siblings: `sidebar (size=28)`, `pane size="50%"`,
`pane size="50%"` — a genuine 2-way even split where the retrofit should
have produced one absorbed slot for the pre-existing pane. This is a live
occurrence, not synthetic: no prior evidence in this entity (ideation's
disposable-session repro, validation's refutation audit) surfaced it — both
only ever observed the clean N→N+1 case.

1. **The doc diff's core claim needs to change or be qualified.**
   `docs/docking-approach.md`/`SPEC.md` currently state the first-toggle
   pane-count change as a clean, singular "N→N+1" exception. That's now
   contradicted by a live repro in the simplest possible case (fresh tab,
   one `Alt /`). Implementation must not simply re-ship the same diff text —
   either the doc needs to accurately describe the real (currently
   unreliable) behavior, or this entity needs to hold for a root cause
   before any doc claim ships. If resolving this properly requires a design
   decision beyond doc text (e.g. it turns out to need a real code fix, not
   documentation), implementation should say so explicitly in its stage
   report rather than force a doc patch that still misdescribes reality.
2. **Likely shared root cause, under active investigation elsewhere.**
   The `WORK` session currently carries 16 leaked zombie floating
   `zellij-sidebar.wasm` instances (12 in "Noteplan", 4 in "CEO") — the
   same ingredient `dock-floating-leak-and-chrome-misplacement` (`eh`)
   already named as its leading, unconfirmed hypothesis for the sibling
   chrome-misplacement bug (a zombie instance racing a legitimate
   retrofit/regenerate). `eh`'s ideation is being redispatched now to trace
   root cause against this exact live state. Coordinate with (or wait on)
   that finding rather than duplicating the investigation here.
3. **Fold in the previously-flagged `README.md:34-37` gap** (see the
   validation Stage Report above) in the same pass, since implementation is
   being reopened anyway.

**Cycle 2 (2026-07-09) — REJECTED at validation, routed to implementation.**

CL rejected the cycle-2 "approve to done" recommendation (a doc correction
that stated the pane count is unreliable without explaining why). Between
cycle 1 and cycle 2's validation, `eh` (`dock-floating-leak-and-chrome-misplacement`)
completed its own ideation cycle 2 and substantially narrowed a root cause:
a check-then-act race with no lock in `install_split_preserving_swaps`
(`dump_contains_sidebar`, `src/main.rs:918-925`, vs. the later
`override_layout` call), with a named fix candidate (serialize per `tab_id`,
or re-check immediately before install) — and critically, `eh`'s own AC-3
already scopes "chrome placement (and pane count) cannot be corrupted by a
concurrent regenerate/retrofit race," meaning `eh`'s fix, once shipped,
directly resolves this entity's original complaint too. CL's objection:
continuing to ship this entity as "document that it's unreliable" undersells
what's now known to be a fixable bug, not an unexplained one.

1. **Do not re-ship another doc hedge.** `eh`'s race-condition theory is
   strong (two independent live occurrences with byte-identical corruption,
   a third independent static trace finding no bug in the transform logic
   itself, and a plugin-id-ordering proof of the spawn mechanism) but is
   itself not yet independently verified outside `eh`'s own investigation
   chain, and `eh`'s ACs remain formally OPEN (not reproduced on demand).
   An adversarial verification pass on `eh`'s three core claims (the race,
   the transform being clean, and the spawn-dedup evidence) is running now
   (workflow `wf_a0149a90-999`) — the FO is holding this entity's cycle-3
   dispatch until that returns, rather than briefing a fix attempt against
   an unverified mechanism.
2. **Open question for cycle 3, not resolved here:** does the actual code
   fix belong in this entity's own worktree, or does this entity stay
   parked (doc-only, honestly caveated) until `eh` ships the fix under its
   own AC-3, at which point this entity's close-out becomes a much simpler
   "state the restored, true invariant" pass? The cycle-3 dispatch will
   settle this once the verification pass reports back.

**Routing correction (2026-07-09, before cycle 3 dispatch).** CL caught a
process error: this rejection was routed to `implementation` per validation's
declared `feedback-to`, but item 2 above asks whether to change the
*approach* (doc-only vs. a real code fix vs. park-and-wait) — an ideation-level
design decision. Per this workflow's own stage contract, implementation's
authority is to build "the approved ideation body," not to choose between
approaches. Cycle 1's ask (correct an overclaimed detail within the
already-approved doc-only approach) was legitimately implementation-level;
cycle 2's was not, and should not have named `implementation` as the next
stage. Status corrected to `ideation` instead. The kept-alive implementation
ensign is being released (no `feedback-to` pointer targets it now).

**Merge decision (2026-07-09, CL confirmed).** An adversarial verification
pass (workflow `wf_a0149a90-999`, 2 independent refuters per claim against
live source, not `eh`'s own report) confirmed `eh`'s narrowed root cause
holds up: the TOCTOU race in `install_split_preserving_swaps` survived both
refuters at high confidence (no lock/mutex anywhere in the file, developers'
own comments acknowledge concurrent instances race the same tab); the
transform functions being clean survived both refuters *empirically*
(temporary tests fed the exact corrupted-shape input, confirmed the code
structurally cannot emit a flat un-wrapped 2-sibling split from one call).
This is a real, fixable code bug, not environmental unreliability to
document around — and `eh`'s own AC-3 already scopes "chrome placement
(and pane count) cannot be corrupted by a concurrent regenerate/retrofit
race," i.e. `eh`'s fix, once designed and shipped, directly satisfies this
entity's AC-1 too.

**This entity is now intentionally parked, not dispatched, pending `eh`.**
No further ideation/implementation work happens here independently — `eh`
carries the actual fix (design, spike-validation, implementation) under its
own AC-1/AC-2/AC-3. Once `eh` ships, this entity's remaining work is a small
closing doc pass: state the now-true restored invariant (no more caveat
about an unreliable count) in `docs/docking-approach.md`/`SPEC.md`/`README.md`,
then gate to `done`. Do not dispatch this entity to any stage until `eh`
reaches its own `done`.

**Cycle 5 gate decision (2026-07-10) — REJECTED; Choice A selected under the captain's conn.** Cycle 5 supersedes the older park-for-`eh` theory: the clean N=2 loss and exact Zellij 0.44.3 server trace prove a separate host-owned failure with neither leaked instances nor chrome corruption present. AC-1 through AC-5 remain unmet in this repository because retained override is nontransactional and original typed launch identity is unavailable. The first officer selected the report's recommended architecture, a transactional result-bearing retained-pane host operation, and filed `zellij-transactional-retained-pane-override` as the blocking task. `j5` remains in ideation until that task supplies the host contract; no repository-only implementation is authorized meanwhile.

## Stage Report: implementation (cycle 2)

- DONE: Resolve the live pane-duplication finding (WORK Tab #7: fresh 1-pane tab + one Alt-/ produced 2 real terminal panes + rail, not 1+rail) -- revise the doc claim to match reality or explicitly hold/defer pending eh's root cause; do not re-ship the disproven "N->N+1" claim unchanged
  Checked `dock-floating-leak-and-chrome-misplacement` first (per feedback, coordinate rather than re-investigate): still `status: ideation`, unmodified since 2026-07-08 16:09, its own AC-1/AC-2/AC-3 all OPEN and its concurrency spike neither confirmed nor refuted the shared-root-cause hypothesis. No new root cause to build a fix on. Revised `docs/docking-approach.md`'s "Pane-count exception" paragraph (commit `a7934cb`) to drop the disproven "N panes + 1 rail" quantification, keep "a new pane is accepted architecture" (still true), state both the clean case (N->N+1, 2026-07-08) and the counter-example (N=1->N=3, `WORK` Tab #7, 2026-07-09) with their evidence, and name the open investigation as the suspected-unconfirmed link. `SPEC.md`'s two sites needed no change — grep confirmed neither quantifies pane count (`grep -n "N panes\|N+1\|N->N\|N → N"` across all three files hit only the one now-revised `docking-approach.md` line).
- DONE: Fold the README.md:34-37 stale "never spawned or hidden" claim into the same doc-diff pass (same fix shape as the other two files)
  Applied the same qualifier pattern SPEC.md landmine #16 already used: scoped the bullet to "once a tab already carries that swap set" and added a sentence naming the first-toggle retrofit as the sole exception, pointing to docking-approach.md's pane-count exception. Committed in the same commit (`a7934cb`).
- DONE: State plainly in the stage report whether this cycle's output is a real fix or a hold-for-eh -- do not paper over an unresolved live bug with confident doc prose
  This is a doc correction, not a root-cause fix, and not a full hold either -- it's the middle case the feedback explicitly allowed ("correct it to match real (currently unreliable) behavior"). The underlying mechanism that produced 2 extra panes instead of 1 in WORK Tab #7 remains unexplained; `eh`'s investigation (the designated owner of that question) has not closed. What shipped this cycle is honest about that: the doc no longer asserts a false universal invariant, states plainly that the count is unreliable, cites both the clean case and the counter-example with evidence, and names the open tracking entity rather than inventing an explanation. If `eh` later confirms a root cause and a code fix lands, this paragraph will need a further revision to describe the fixed behavior -- this cycle does not close that loop.

### Summary

Re-opened after Cycle 1 rejection (CL's live WORK Tab #7 repro: N=1->N=3, not the shipped N=1->N=2 claim). Checked the concurrent `dock-floating-leak-and-chrome-misplacement` investigation first per feedback instruction -- it's still open/unconfirmed, so no root cause exists to fix against. Revised `docs/docking-approach.md`'s pane-count-exception paragraph to drop the disproven "+1" quantification while keeping the still-true "a new pane is accepted architecture" claim, citing both the clean case and the live counter-example and naming the open investigation as the suspected (unconfirmed) shared cause; folded in the previously-flagged `README.md:34-37` stale claim with the same qualifier shape already used in `SPEC.md`. `SPEC.md` itself needed no change (neither site quantifies pane count). Committed to the worktree branch as `a7934cb`. `cargo check --tests` and `cargo test` both clean (130/130) confirming this remains doc-only. This is a doc correction that stops the entity from shipping a disproven claim, not a fix for the underlying extra-pane mechanism, which stays open pending `eh`'s root cause.

## Stage Report: validation (cycle 2)

- DONE: Independently re-diff commit a7934cb across docking-approach.md, SPEC.md, and README.md against the stage report's claims -- confirm the revision actually drops the disproven universal "N->N+1" claim and accurately states both the clean case (2026-07-08) and the WORK Tab #7 counter-example (N=1->N=3, 2026-07-09) with citations, not just that some diff landed
  `git show a7934cb` confirms the diff touches only `README.md` and `docs/docking-approach.md` (SPEC.md untouched, matching the report's claim neither site quantifies count). Byte-compared the entity's drafted "after" text for each of the three hunks (docking-approach.md invariant-sentence replacement, docking-approach.md pane-count-exception paragraph, README.md bullet) against the actual shipped file text in a throwaway detached checkout of `a7934cb` -- all three identical (only diff was the drafted text's markdown code-fence marker, not file content). The shipped pane-count-exception paragraph states the clean N->N+1 case (2026-07-08) and the N=1->N=3 WORK Tab #7 counter-example (terminal_41/terminal_42, 2026-07-09) side by side, with the "+1" quantification dropped from the lead sentence, exactly as claimed.
- DONE: Refutation audit on a throwaway checkout (never the implementation worktree) -- grep all three touched/considered files for any remaining unconditional or quantified pane-count claim the revision might have missed, and re-confirm AC-4 (chrome misplacement) is still correctly left open, not implicitly resolved by the reworded pane-count language
  Used `git worktree add --detach /tmp/spacedock-validation-throwaway a7934cb` (removed after use). Grepped all three files for `N+1`, `N->N`, `N panes + 1`, and unconditional `never creates/spawned/hidden/shown/moved/destroyed` language: the only quantified hits are the two intentionally-scoped sentences in the new pane-count-exception paragraph (clean case + counter-example, both correctly hedged, not asserted as universal); the only unconditional-language hit is the already-fixed, correctly-scoped invariant sentence ("every toggle after a tab carries the swap set... never creates, hides, shows, moves, or destroys a pane"). Broader sweep for `always`/`exactly` near pane/toggle/dock language turned up only unrelated claims (fidelity-ceiling "panes and content always survive," chrome-row-count invariants, election precedence) -- none re-assert the disproven first-toggle count. `git show a7934cb` diff has zero mentions of `chrome`, `tab-bar`, `status-bar`, or `extract_chrome_panes` in changed lines; the entity body's AC-4 section, "Proposed doc diff" preamble, and "Out of scope" section are all untouched by this commit (confirmed via `git show 76c614b` on the state checkout) and still state AC-4 open.
- DONE: Sanity-check the revised/expanded Acceptance criteria section (grew 64->76 lines this cycle) -- each caveat is evidence-backed and consistent with the entity's actual current claims, not aspirational or overstated
  Confirmed the exact growth by diffing `## Acceptance criteria` section boundaries before/after commit `76c614b` on the state checkout: 64 lines (215-278) before, 76 lines (215-290) after -- matches the checklist's stated growth precisely. The only content added to the section this cycle is the 12-line AC-1 caveat citing the WORK Tab #7 counter-example; its two factual claims were independently checked against the codebase and the Feedback Cycles section: (1) the terminal IDs/N=1->N=3 figures match the Feedback Cycles entry verbatim; (2) the claim that "that fixture test was never actually built" is accurate -- grepped `src/main.rs` for existing `split_preserving_layout_kdl` tests and read the closest match (`split_preserving_layout_wraps_the_dumped_arrangement_in_both_swaps`, main.rs:3751); it exercises a single fixed dump fixture, not a parameterized N=1-vs-N=3 comparison, so no test currently proves or disproves the KDL-generation N-independence claim -- the caveat's "untested either way, not disproven" framing holds. Not aspirational or overstated.

### Summary

Independently re-verified commit `a7934cb` on a throwaway detached checkout of the implementation worktree's exact commit (never touching the worktree itself): all three files' shipped text matches the entity's drafted diff byte-for-byte, the disproven universal "N->N+1" claim is gone, and both the clean case and the WORK Tab #7 counter-example are stated with matching citations. The refutation audit found no remaining unconditional or quantified pane-count claim anywhere in the three files beyond the two intentionally-scoped sentences this revision added, and confirmed AC-4 remains untouched and open. The Acceptance criteria section's growth (64->76 lines) is entirely the one AC-1 caveat, whose two factual claims (WORK Tab #7 figures, absence of an N-independent fixture test) both check out against source. This revision is honest: it stops asserting a disproven invariant without overclaiming a fix for the underlying mechanism, which stays correctly open pending `eh`'s investigation. Recommend gate-approval to done.

## Stage Report: ideation (cycle 3)

- DONE: Replace the race/shared-root-cause premise with the confirmed Run::Cwd versus bare-slot mechanism, citing WORK's serialized template, Zellij's exact matcher, and the one-call CLI reproduction.
  The canonical Problem traces `Some(Run::Cwd("/Users/clkao"))` versus the base slot's `None` through `find_already_running_panes`; one explicit-cwd retained override reproduced `terminal_0 + terminal_1 + rail`, while the bare-pane control stayed `terminal_0 + rail`.
- DONE: Specify the smallest fix spike and regression proof: an explicit-cwd existing terminal, exactly one retained override call, exactly one terminal plus rail afterward, with a bare-pane control.
  Four disposable Zellij 0.44.3 sessions compared current and no-leaf bases; the no-leaf base produced `terminal_0 + rail` for both explicit-cwd and bare identities, preserved cwd, and required one active-tab override call per session. All sessions were torn down; WORK was not mutated.
- DONE: Rewrite the acceptance criteria, test plan, proposed doc diff, and scope boundary so j5 owns terminal duplication while eh retains the floating-leak/chrome investigation; do not implement production code in ideation.
  The canonical body now defines structural and process-level regression proofs, concrete README/SPEC/docking-approach changes, and an explicit eh boundary including the plugin_84 status-bar-in-rail evidence. The code worktree remains clean; prior reports and false conclusions are retained under Superseded ideation and validation history.

### Summary

The first-toggle extra terminal is now a deterministic launch-identity bug, not an accepted retrofit cost or a concurrency symptom. A live server-level spike validated the smallest design: remove every spawnable terminal leaf from the override base, retain the real terminals, and let the installed swaps re-seat them; implementation remains for the next stage. j5 owns that fix and its regression/docs, while eh continues to own floating-instance leakage, races, and chrome placement.

## Stage Report: ideation (cycle 4)

- DONE: Treat the exact two-terminal stacked case as a refutation of the no-terminal-leaf safety premise.
  A clean disposable 160x60 session reproduced WORK's terminal loss after one retained override: `terminal_0 + terminal_1` became `terminal_0 + rail`, with `UnsatisfiableConstraint` and “Not enough room for another pane”; corrupted chrome and concurrency were absent.
- DONE: Determine the smallest safe next investigation/design.
  No production layout rewrite is approved. The next spike must find a non-spawning transactional retained-pane insertion point; only if absent may it investigate original-`invoked_with()` readback for an identity-complete concrete base. Exact concrete runs preserved N=2, but launch-cwd A/current-cwd B refuted using dump/current cwd as identity.
- DONE: Update ACs, test plan, and docs claims for multi-terminal safety and failure-path proof.
  Canonical ACs now require exact terminal-ID preservation for N=1/N=2/N=3, non-destructive unsatisfiable relayout, launch-identity drift, and bare controls; the cycle-3 no-leaf doc diff is withdrawn and retained under a refuted-history section.
- DONE: Do not mutate WORK or implement production code.
  WORK was read only for pane/layout evidence; every experiment used disposable sessions that were torn down, all temporary KDL was removed, and the code worktree is clean.

### Summary

Cycle 4 replaces a premature N=1 fix with a safety gate grounded in the N=2 loss path. j5 remains in ideation until the server can seat arbitrary retained terminals safely before swap relayout, or can expose their original launch identities; otherwise the task returns to the captain as a host-API or architecture decision. eh continues to own floating-instance concurrency and chrome placement.

## Stage Report: ideation (cycle 5)

- DONE: Exercise Zellij's server/API path for a non-spawning, transactional retained-pane insertion point that seats N>=2 terminals before swap application; provide the smallest end-to-end proof or exact source/API evidence that the mechanism is unavailable.
  Exact v0.44.3 source traces retained panes through drain -> remove -> `insert_pane`, whose no-room branch logs and drops the owned pane without rollback; `children` produced one bare spawnable slot in a disposable N=2 session, and a flexible plugin anchor's N=2 success still used the same destructive branch.
- DONE: If no insertion point exists, trace whether each terminal's original invoked_with identity can be read and round-tripped for launch-cwd drift, bare panes, and command panes without substituting current cwd; falsify unsafe candidates with disposable evidence.
  `PaneInfo` lacks typed identity, cwd/running-command calls return current OS state, and session dump overwrites original command/cwd before serialization; bare, cwd-drift, and current-command concrete bases each duplicated terminal `{0}` to `{0,1}`.
- DONE: Update the canonical design, ACs, and test plan with one proved safe mechanism or an explicit host-API/architecture blocker; do not edit production code or mutate WORK.
  Canonical design now records the blocker and a two-choice captain gate: transactional Zellij host support (recommended) or a reopened foreign-tab docking architecture. No production code, docs, or WORK state changed.

### Summary

Zellij 0.44.3 exposes neither an atomic retained-pane override nor original typed launch identity, so j5 has no safe repository-only implementation. The next step requires a captain decision between patching/upstreaming a transactional host operation and changing the first-toggle architecture for foreign tabs.
