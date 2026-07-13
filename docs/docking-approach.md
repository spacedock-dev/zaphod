# Docking via layouts — historical prototype findings

> **Historical prototype record — not an operational contract.** This document
> records the earlier Zellij WASM dock and current-tab retrofit experiments.
> They are superseded: create a selected-checkout managed tab with
> `scripts/zellij-new-tab.sh`. A separately installed `Alt Shift z` shortcut
> opens only its fixed configured layout. Persistent `Alt /` is `NoOp`, and an
> active tiled rail may temporarily route it to its own plugin id. `Alt /`
> never creates or retrofits a tab. The evergreen direction uses one managed
> tab or window and is defined in
> [`docs/zaphod-workspace-architecture.md`](zaphod-workspace-architecture.md).
>
> Investigation date: 2026-06-20 · shipped prototype validated 2026-07-02
> zellij CLI 0.44.1 · `zellij-tile` locked at 0.44.3
> Scope: replace the runtime `embed_multiple_panes` + resize-hysteresis dock with a
> layout-driven docked tile. Supersedes the "runtime tiled docking is unwinnable"
> verdict in `SPEC.md` (landmines #10/#11).

## Problem

The sidebar has two presentation modes:

- **Floating pinned rail** — `float_as_rail` / `rail_coordinates` (`src/main.rs:277`,
  `:492`). x=0, fixed width, ~97% height, pinned. The only runtime-exact placement,
  but it **overlays** — it does not reserve space, so the underlying panes keep their
  full width and the rail sits on top of them. The rail machinery is deleted under
  the shipped prototype architecture (below).
- **Docked tile** — the mode the user actually wants: a left column that **reserves
  space**, pushing the other panes aside.

The docked tile was implemented at runtime via `embed_multiple_panes` (let the server
pick a tiled slot) followed by a shrink-only resize loop (`fn dock`, `src/main.rs:354-381`).
That path is unreliable for structural reasons, all four of which were observed:

1. the plugin does not choose the slot (`embed_multiple_panes`, `src/main.rs:390-391`)
   — a full-width pane lands at the bottom/right, not a left rail;
2. the shrink loop only changes width, never position, and latches `docked=true` on the
   `MAX_DOCK_STEPS` give-up cap (`src/main.rs:11`, `:372-380`);
3. tiled resizes trigger auto-layout reflows that restack the whole tab (SPEC #15);
4. it never reserves space cleanly across an existing tab of agents.

The reliable layout-based dock that `SPEC.md` points at (`default_tab_template` /
`new-tab --layout`) is partly wired: `~/.config/zellij/layouts/default.kdl` already
docks the sidebar statically in every new tab. What was missing is the **toggle** —
flipping that reserved column off and back on at runtime.

## What was validated

### The `override_layout` API (static — high confidence)

`zellij-tile` exposes a runtime layout-override the SPEC predated:

- **Present in the linked version.** `Cargo.lock` resolves `zellij-tile = "0.44.1"` up to
  **0.44.3**, whose shim has `override_layout`. No dependency bump required.
- **Signature** (`zellij-tile`/`zellij-utils` 0.44.3):

  ```rust
  override_layout(
      layout_info: impl AsRef<LayoutInfo>,
      retain_existing_terminal_panes: bool,
      retain_existing_plugin_panes: bool,
      apply_only_to_active_tab: bool,
      context: BTreeMap<String, String>,
  )
  ```

- **Inline KDL is a first-class input** via `LayoutInfo::Stringified(String)`
  (`zellij-utils/src/data.rs`, enum `LayoutInfo { BuiltIn, File, Url, Stringified }`).
  A plugin builds a layout from a raw KDL string with no on-disk file.
- **Non-blocking shim** — implemented with `object_to_stdout` + `host_run_plugin_command`,
  no `bytes_from_stdin`. Safe to call from `pipe()` / `update()` / `load()`, unlike the
  blocking shims behind SPEC landmine #4.
- **CLI mirror** for testing: `zellij action override-layout --layout-string '<KDL>'
  --apply-only-to-active-tab --retain-existing-terminal-panes
  --retain-existing-plugin-panes`. Same server-side `OverrideLayout` path as the plugin call.
- **`next_swap_layout()` / `previous_swap_layout()`** exist in 0.44.1 and are likewise
  non-blocking.

### Live CLI validation (the proof — and the mess)

Ran the CLI `override-layout` with a bare two-slot left-rail KDL against a live attached
tab containing many agent panes. The `list-panes -g` before/after showed:

- ✅ **Tiled space reserved** — a new pane appeared at **X=0, COLS≈28**; the other panes
  shifted to **X≈28**. The floating rail can never do this. Runtime tiled docking is real.
- ✅ **Existing panes retained** — every prior `terminal_*` survived (relocated, not
  respawned).
- ✅ **Blast radius scoped** — only the active tab re-tiled; the other ~5 tabs were
  unchanged. (`--apply-only-to-active-tab` works.)

…but the active tab also became messy, in three ways that share one root cause:

- empty `pane` slots in the layout **spawned new shells** (unwanted panes);
- the layout string omitted the `zellij:tab-bar` / `zellij:status-bar` panes, so the
  override rebuilt the tab without them and **the tab header dropped to the bottom**;
- the retained agents had no clean home in a two-slot layout, so they were **crammed**.

### What `override_layout` actually does

**It replaces the entire active-tab layout — it does not add a rail.** The new layout's
slots are materialized (empty leaves spawn shells; omitted chrome is dropped) and the
existing panes are kept *in addition* as "retained" extras when the retain flags are set.
This makes it:

- the right tool for **establishing a known, complete arrangement** (must include the
  tab-bar/status-bar chrome and a slot structure that absorbs the existing panes);
- a **blunt instrument for retrofitting** an arbitrary live tab — hence the mess above.

### KDL gotchas confirmed live

- `pane stacked=true` **must** have child nodes, or a `children` node when inside a swap
  layout — an empty `stacked=true` fails to parse
  (`A stacked pane must have children nodes…`).
- Any layout applied with `override_layout` must reproduce the **full** intended tab,
  including `pane size=1 borderless=true { plugin location="zellij:tab-bar" }` and the
  matching `zellij:status-bar`, or that chrome is lost.

## Revised platform verdict (SPEC landmine updates)

| SPEC landmine | Revised status (zellij 0.44.0+) |
|---|---|
| #10 runtime tiled placement "unwinnable" | **Obsolete for docking.** Low-level `move_pane` is still swap-based, but `override_layout` applies a declared tiled layout to the active tab. |
| #11 layouts are the only reliable tiled placement | **Refined.** Still true that *layouts* place tiles — but a layout can now be applied at runtime (inline KDL via `override_layout`), not only at tab creation. |
| #12 floating + `change_floating_panes_coordinates` is the only runtime-exact placement | **Still true** — and remains the right tool for the on-demand floating summon. |
| #15 tiled resize/float transitions restack the whole tab | **Mitigated.** `apply_only_to_active_tab` scopes the reflow to one tab; `retain_existing_*` preserves the user's panes. |

## Proposed approach

A two-mode split, mirroring how yazelix solves the same problem (swap layouts, not
runtime embed/resize):

### Primary — swap-layout docked tile

Start the agent workspace tab **from a zaphod layout** that ships a docked and an undocked
swap. `⇄` calls `next_swap_layout()` / `previous_swap_layout()`. Swap layouts rearrange the
tab's **existing** panes between states **without spawning shells or touching chrome** —
which is exactly the cleanliness the `override_layout`-on-live-tab path lacks.

Proposed layout files (sketch — syntax to be verified against 0.44 swap-layout grammar;
absolute wasm path substituted at install time):

`~/.config/zellij/layouts/zaphod.kdl`
```kdl
layout {
    default_tab_template {
        pane size=1 borderless=true { plugin location="zellij:tab-bar" }
        children
        pane size=1 borderless=true { plugin location="zellij:status-bar" }
    }
    // docked: 28-col plugin rail on the left, agents stacked on the right
    pane split_direction="vertical" {
        pane size=28 borderless=true { plugin location="file:/ABS/PATH/zellij-sidebar.wasm" }
        pane stacked=true { children; }
    }
    swap_tiled_layout name="undocked" {
        ui {
            pane stacked=true { children; }
        }
    }
    swap_tiled_layout name="docked" {
        ui {
            pane split_direction="vertical" {
                pane size=28 borderless=true { plugin location="file:/ABS/PATH/zellij-sidebar.wasm" }
                pane stacked=true { children; }
            }
        }
    }
}
```

### Fallback — `override_layout` retrofit

For a tab **not** started from the zaphod layout (no swap set loaded), the only runtime
option to reserve space is `override_layout(LayoutInfo::Stringified(kdl), true, true, true,
{})`. The KDL **must** include the tab-bar/status-bar chrome and a stacked main that
absorbs the existing agents. Accept that this is a full-tab replacement and is less clean
than the swap path.

### Summon — floating pinned rail (superseded)

This proposal kept `float_as_rail` / `rail_coordinates` for an on-demand summon in tabs
that lack the layout (SPEC #12). The shipped prototype architecture (below) supersedes it: the
sidebar exists in every tab's layout, the retrofit override covers tabs without the
swap set, and the summon machinery is deleted.

### Concrete code changes

- Add layout assets + an install step (extend `build.sh` or add `install.sh`) that
  materializes `zaphod.kdl` (+ swap layouts) to `~/.config/zellij/layouts/` with the
  absolute wasm path substituted. The directory is currently empty.
- Rewire the `⇄` header toggle (`handle_click`, `src/main.rs:386-394`) from
  `embed_multiple_panes` + resetting `dock_steps` → `next_swap_layout()` /
  `previous_swap_layout()` when the manifest shows the pane is tiled in a swap-layout tab;
  fall back to `override_layout` for the retrofit case.
- Retire the runtime dock machinery once the replacement is validated:
  `fn dock` (`src/main.rs:354-381`), the `self.dock(cols)` render call (`src/main.rs:255`),
  the `embed_multiple_panes` arm, and the `dock_steps` / `docked` / `TARGET_COLS`
  (`src/main.rs:10`) / `MAX_DOCK_STEPS` (`src/main.rs:11`) state. Fixed width becomes the
  layout's `size=28`.

## Open questions / what still needs a live test

These need a focused, attached session (the headless `--create-background` bench has no
active client, so `--apply-only-to-active-tab` silently no-ops there — a bench limitation,
not a plugin one):

1. **Swap-layout absorption.** Do existing agent panes flow into the docked swap's
   `stacked=true { children; }` main without spawning new shells? (The expected, but
   unverified, advantage of swap layouts over `override_layout`.)
2. **Plugin-instance reuse across a swap.** When the undocked swap omits the sidebar slot
   and the docked swap restores it, is zaphod's existing instance reused, or destroyed and
   respawned (re-incurring the permission / first-render flow)? If respawned, keep the
   plugin pane in both swaps and vary only its width/visibility.
3. **Chrome preservation** under swap layouts (does the tab-bar/status-bar survive a
   docked↔undocked cycle cleanly).
4. **Swap-layout KDL grammar** for 0.44 (`swap_tiled_layout` names, `children` placeholder,
   `min_panes`/`max_panes` selectors) — the sketch above is unverified.

Smallest end-to-end test (mechanism-validation-first): start a tab with
`zellij action new-tab --layout zaphod`, open several panes, bind a key to a pipe that
calls `next_swap_layout()`, and confirm it cycles docked↔undocked, that the plugin pane
survives the swap without reload, and that no stray shells appear.

## Validation status

| Item | Status | Evidence |
|---|---|---|
| `override_layout` in linked version | ✅ verified | `Cargo.lock` → `zellij-tile` 0.44.3; shim present |
| Inline KDL via `LayoutInfo::Stringified` | ✅ verified | `zellij-utils/src/data.rs` enum |
| Non-blocking (safe from `pipe()`) | ✅ verified | shim uses `object_to_stdout`, no `bytes_from_stdin` |
| Reserves tiled space at runtime | ✅ verified live | CLI override → pane at X=0 COLS≈28, others shifted to X≈28 |
| Retains existing panes | ✅ verified live | all `terminal_*` survived |
| `--apply-only-to-active-tab` scopes blast radius | ✅ verified live | other tabs unchanged in `list-panes -g` |
| `override_layout` = full active-tab replacement | ✅ verified live | empty slots spawned shells; omitted chrome dropped header to bottom |
| Swap-layout absorption (no spawned shells) | ✅ verified live | docked↔undocked cycle re-tiled the existing panes, no new shells |
| Plugin survives the swap (R4) | ✅ verified live | undock absorbed the sidebar into the stack (collapsed to a title row), re-dock restored it to the rail — same instance, no respawn / re-permission |
| Swap-layout KDL grammar | ✅ verified live | `swap_tiled_layout { tab { … children … } }` — `tab {}`, not the sketch's `ui {}`; inline `{ plugin … }` fails to parse, use multi-line |

## Outcome (validated live & shipped, 2026-06-20)

**Approach A (hide/show the docked layout instance) — disproven.** Toggling a
docked tile via `hide_self()` / `show_self()` does not restore the rail:
`show_self` re-inserts the pane next to whatever is focused (observed: buried
bottom-right at the wrong size), not into its layout slot. Same unreliability
class as the runtime dock.

**Approach B (swap layouts) — shipped.** `Alt /` → the plugin's `toggle` pipe →
`next_swap_layout()` when the active-tab instance is a docked (tiled) sidebar.
A zaphod agent-tab layout (`layouts/zaphod.kdl`, installed via `install.sh`)
ships `docked` + `undocked` `swap_tiled_layout`s; a tab created from it carries
only those two, so `next_swap_layout()` is a clean 2-state toggle (verified: the
cycle never touched the session's vertical/horizontal/stacked swaps). The
runtime-dock machinery (`fn dock`, `embed_multiple_panes`, the
`dock_steps`/`docked`/`TARGET_COLS`/`MAX_DOCK_STEPS` state) is retired; the `⇄`
header control also calls `next_swap_layout()`. Floating-rail summon was kept at
the time for tabs without a docked instance; the shipped prototype architecture (below)
deletes it.

**Caveats found during live validation:**

- The compiled-plugin cache is path-keyed; a changed *configuration identity*
  did **not** bust it (contra SPEC #1's escape-hatch note). A rebuilt binary at
  the same path needs `skip_cache true` or a fresh session to load.
- Permission grants did not persist to `permissions.kdl` across config
  identities in testing — each fresh plugin identity re-prompted.
- Keybind → docked-instance routing depends on the `Alt /` keybind config
  (`rail "1"`) matching the layout plugin's configuration identity. The layout
  carries `rail "1"`; confirm with a real `Alt /` keypress in a fresh session
  (a CLI `zellij pipe` with a mismatched `--plugin-configuration` spawns a new
  floating instance instead of reaching the docked one).

## Historical prototype architecture (validated live, 2026-07-02)

The retired end state, mirroring yazelix's model:

**The sidebar pane existed in every toggled tab's layout, permanently.** The
retired default layout was chrome-only; its first toggle retrofitted the
sidebar in.
"Toggle" (`Alt /` and the `⇄` header control) never creates, hides, shows,
moves, or destroys a pane — it only cycles the tab's `swap_tiled_layout`
states:

- **docked** — the sidebar reserves a left column (`size=28`);
- **undocked** — the sidebar collapses to a separate `size=1` sliver
  (yazelix-style), never absorbed into the stacked main.

`hide_self` / `show_self` / `open_plugin_pane_floating` are never called; the
whole summon/spawn machinery is deleted (list below).

Two paths put the swap set on a tab:

1. **Layout-born tabs** (`zellij action new-tab --layout zaphod`) carry the
   docked/undocked swap set from birth. Toggle = `next_swap_layout()`. The
   default layout stays chrome-only — tab-bar / `children` / status-bar plus
   an explicit `tab { pane }` (a template-only or bare-`tab` layout births
   zero terminals and the session exits immediately; both observed live) —
   so ordinary tabs are born without a sidebar and reach the docked state
   via the retrofit.
2. **Retrofit** — for a live tab without the swap set, a one-time
   `override_layout(LayoutInfo::Stringified(kdl), retain_terminals=true,
   retain_plugins=true, apply_only_to_active_tab=true)` whose KDL contains the
   tab-bar/status-bar chrome, the `rail "1"` sidebar slot, and **both**
   `swap_tiled_layout` sections installs the swap set on that tab; every
   subsequent toggle is pure swap cycling (no per-toggle re-override). The KDL
   is generated from the tab's own dumped arrangement so the user's splits
   survive the retrofit — the split-preserving rebuild detailed under Toggle
   v3 below, not a blind stacked-absorb. Any instance that observes a
   sidebar-less active tab retrofits it (the relaxed election, arc entry
   below); an instance that cannot run the rebuild defers rather than
   absorbing.

### Live validation evidence (2026-07-02, ztest, zellij 0.44.1, attached client)

The one unproven mechanism — does an override KDL *containing* swap sections
install that swap set on the tab — was confirmed:

| Step | Observed |
|---|---|
| Control probe: `next-swap-layout` pre-override | builtin compact swap set cycled — the baseline signature a refuted result would reproduce |
| `override-layout --layout-string "$(cat …)" --apply-only-to-active-tab --retain-existing-terminal-panes --retain-existing-plugin-panes` | exit 0; tab renamed; 28-col sidebar docked left; existing panes reflowed into the stacked main; chrome replaced; other tabs untouched |
| `next-swap-layout` press 1 | **no visible change** — the base layout is geometrically identical to the "docked" swap; the toggle UX must account for this silent first press |
| press 2 | sidebar collapsed to the 1-col sliver |
| press 3 | restored to the 28-col rail; same plugin instance throughout (no reload, no permission re-prompt, no new shells) |
| Leak check: one press with another tab focused | nothing happened — the swap set installs **per-tab**, not session-wide |

Caveats from the run:

- **Absorption fidelity:** three pre-existing panes landed as a stack-of-2 plus
  one standalone sibling rather than one stack of three. Minor UX wart, not a
  mechanism failure.
- **No read-back probe exists:** `dump-layout` omits *per-tab* swap sets — both
  installed-by-override and carried-from-`new-tab --layout` ones (verified
  live; the session-level swap set does appear in the dump) — so per-tab
  swap-set installation can only be verified behaviorally.
- The run auto-granted from the cached grant — the grant cache lives at
  `~/Library/Caches/org.Zellij-Contributors.Zellij/permissions.kdl` (not
  Application Support).

### Toggle v2 live evidence (2026-07-02 evening, fresh ztest, attached client)

Source cites below are zellij v0.44.1.

**dump→transform→override — REFUTED.** The candidate split-preserving toggle
(dump the tab's actual layout, flip the sidebar width, override the same tab)
fails structurally. Round-trip experiment on two tabs, identical signature:
overriding a tab with its own *unchanged* dump spawned three new shells and
nested the original three panes as leftovers. Root cause: `dump-layout` emits
a **resurrection** format — a concrete `pane` node means "spawn a new pane
here"; retained panes re-seat only into `children` insertion points
(`layout_applier.rs:160-221`). A dump cannot round-trip through
`override-layout` on the same tab. Consequences: the toggle stays pure swap
cycling (made deterministic below), and the retrofit KDL keeps its
`stacked=true { children }` main — the only retained-pane-correct shape,
proven three times. Positive datum from the same run: the floating sidebar
instance seated cleanly into a config-matched floating slot of an override
KDL (no duplicate) — the retain flags plus exact configuration identity do
re-seat plugin panes. The "deterministic re-stack is the ceiling"
conclusion this implied is superseded: concrete pane nodes spawn only under
*override/tab* application, while **swap** application re-seats them — the
mechanism Toggle v3 (below) is built on.

**Retired retrofit arm — VERIFIED.** With the session's sole sidebar instance
floating in another tab, `Alt /` on a sidebar-less tab ran the cross-tab
election (lowest pane id acted), the override hit the *active* tab rather
than the actor's, the unnamed `tab` node preserved the tab's name, and
post-retrofit toggling cycled cleanly.

**Retired bootstrap gap — CONFIRMED.** In a fresh chrome-only session, the
then-`Alt /` keybind's launch-if-missing spawned a
*floating* rail-"1" instance in the active tab. `own_tab == active` then
routed every toggle to `next_swap_layout()` on a tab with no zaphod swap
set: toggle-dead, and the resident blocked the retrofit election.
`decide_toggle` now carries the floating dimension — a floating resident
retrofits its own tab; a tiled sidebar in the tab always takes precedence,
so the state settles whether or not the override seats the floating actor
into the rail slot. (Whether it seats is the remaining live question; the
positive datum above says a config-matched slot should claim it.)

**Deterministic cycling — designed from source, needs live confirmation.**
Two mechanisms make a single `next_swap_layout()` per press nondeterministic:

- `set_base_layout` inserts every tab's birth layout as swap position 0,
  named "BASE", constrained `ExactPanes(birth pane count)`
  (`swap_layouts.rs:38-57`). The installed cycle is really
  `[BASE, docked, undocked]`; BASE drops out of fit when the pane count
  changes, so the cycle length varies at runtime. This — not just
  geometry — is the silent-first-press mechanism.
- A damage latch: any manual split, user resize, terminal-window resize, or
  added tiled pane sets `is_tiled_damaged`; the next call then only
  re-applies (snap-folds) the current template *without advancing*
  (`swap_layouts.rs:242-251`).

Plugins see both through `TabInfo.active_swap_layout_name` /
`TabInfo.is_swap_layout_dirty` (`data.rs:2242-2244`), so the toggle now
issues **two** `next_swap_layout()` calls on a dirty tab and one on a clean
tab. Caveat: zellij reports `(None, false)` for a tab with at most one
selectable tiled pane (`tab/mod.rs:1005-1018`), so damage there is invisible
and that tab keeps the fold-then-flip behavior. Toggle v3 (below) demotes
the dirty-tab two-call cycle to a fallback: the primary dirty-tab path
regenerates the swap set around the tab's current arrangement, so manual
splits survive the toggle instead of snap-folding.

**Resident-without-swap-set detection — REFUTED.** A tiled sidebar in a tab
without the zaphod swap set (e.g. a pre-rollout captured template) still
dead-cycles, and `active_swap_layout_name` cannot detect the case: a
no-swap-set tab reports `Some("BASE")` — every tab gets the BASE entry —
which is exactly what a freshly-born or freshly-retrofitted zaphod tab
reports before its first toggle (`set_swap_tiled_layouts` +
`set_base_layout` reset the position to 0, `swap_layouts.rs:58-62`,
`tab/mod.rs:923-930`). Routing "BASE" to the retrofit would fire a
destructive override on every fresh zaphod tab. Still open.

### Toggle v3 — split-preserving swaps regenerated at toggle time (mechanism validated live 2026-07-03, ztest; plugin implementation awaiting live validation)

CL's design idea removes the re-stack ceiling: generate the docked/undocked
swap templates **from the tab's current arrangement at toggle time**, so
cycling re-seats the user's own layout with only the rail width changed.
The core bet — concrete-slot **swap** layouts re-seat existing panes,
unlike override/tab application, which spawns new panes for concrete nodes
(the 2026-07-02 refutation above) — was confirmed live.

**The falsifiable rig.** A first rig with a single fitting swap was
unfalsifiable: zellij auto-applies the only fitting swap whenever a pane
opens, so every press looked like a no-op. The proving rig used a
layout-born tab whose base was chrome + `stacked { children }` plus two
chrome-carrying *concrete* swaps — "expanded" (left pane 75%) and
"collapsed" (left pane 25%), both with a 50/50 right column — and three
shells running `while true` echo loops. Cycling visibly flipped the left
pane 75↔25 with the **same panes re-seated** each time: same cwds, all
three loops printing through multiple cycles, and zero pty spawn events in
the server log across every press (each spawn line mapped to a tab
creation, none to a swap). Swap application never spawns (PR #2167
semantics, now witnessed), and re-seating never respawns terminals — agent
processes survive toggling. Unconfirmed: whether scrollback survives the
fold/1-row transitions; shell-history correctness is *not* process-survival
evidence (fresh shells read `~/.zsh_history` too).

**The chrome-wedge lesson.** A control tab with a bare `tab { pane }` base
under chrome-carrying swaps wedged permanently: fold-to-BASE re-seated the
shells into the chrome plugins' 1-row borderless slots (destroying the
tab-bar/status-bar panes), after which both swaps were unfittable forever —
swaps never spawn, and the plugin nodes had no surviving panes to match.
Fold-to-BASE also ignores pane-count fit (it crammed three panes into a
one-slot base) and is destructive. Law: **the base must carry exactly the
same chrome as the swaps.** The implementation goes one further: generated
layouts carry exactly one canonical chrome row per chrome pane the dump
shows the tab actually has — extracted from wherever absorb seated it,
since a baked-in copy carried verbatim rides into the swaps as a user pane
(observed live: tab-bar inside a quadrant, inside a stack) — so base,
swaps, and live tab are chrome-consistent by construction. The
server logged non-fatal "Can't combine fixed panes" / "Failed to find
position of flexible pane" (`tiled_pane_grid.rs:2153`) during the wedge —
fingerprints of a chrome-destroyed fold, harmless but noisy.

**The position quirk → steer, never blind-cycle.** With
[BASE (unfittable), expanded, collapsed] installed, next-spam stuck/no-oped
around the end of the list, while a later next and a previous both stepped
cleanly. Source explains it: `next` past the last entry resets the position
to 0 *without applying*, while `previous` from 0 wraps deterministically to
the last entry (`swap_layouts.rs` `swap_tiled_panes` progress macro). Design
rule: read `TabInfo.active_swap_layout_name` and issue **one deliberate
next or previous per press**. With the installed order
[BASE, docked, undocked], every steering move — docked→undocked (next),
undocked→docked (previous), BASE→docked (next), BASE→undocked (previous,
clean 0→end wrap) — avoids the flaky next-past-end zone entirely.

**The v3 toggle (implemented in `src/main.rs`, wasm not yet rebuilt/validated live):**

- **Clean tab, tiled resident** — one steered press by name, per the rule
  above. Only "docked" steps forward (to undocked); everything else —
  undocked, BASE, foreign, absent — steps backward. BASE is geometrically
  identical to docked on template-born tabs, so a forward step from BASE
  is a dead press (observed live); backward from position 0 wraps
  deterministically to undocked, a visible collapse.
- **Dirty tab, tiled resident** — `dump_session_layout_for_tab(tab_id)`
  (response-carrying, in-band errors, 1s server-side timeout,
  `ReadApplicationState`), then a pure transform
  (`split_preserving_layout_kdl`) builds the override KDL: base = dumped
  chrome + rail slot + `stacked { children }` (the only
  retained-pane-correct override shape), swaps docked/undocked = dumped
  chrome + rail (28/1) + the dumped arrangement with `focus=true` and
  `name` attributes stripped, sizes/`split_direction`/stack flags/cwds
  kept, and `floating_panes` dropped. The server strips the requesting
  plugin's own pane from the dump (`remove_plugin_from_layout`), so the
  rail arrives pre-removed. Then
  `override_layout(Stringified, retain, retain, active-tab-only)` and one
  steered press to the target state. Cost: a momentary stack flash while
  the base applies. Count drift after the toggle: stale generated swaps
  stop fitting and are skipped; the next dirty toggle regenerates.
- **Deferred press.** The override is dispatched on a spawned server thread
  (`run_action`, `zellij_exports.rs:1421`), while `next/previous_swap_layout`
  route synchronously — an immediate press can race the override and cycle
  the *old* swap set. The press is therefore recorded with its **target
  dock state** and resolved on a later `TabUpdate`, honoring two laws
  learned live (drill 7) and confirmed in source:
  - **A landed override does not sit at BASE.** `tab.override_layout`
    installs the set, applies the base, then immediately relayouts with
    the damage flag set (`tab/mod.rs:978-981`): the tab advances to the
    first *fitting* entry. BASE's constraint is
    `ExactPanes(base template leaf count)` — chrome 2 + rail 1 +
    children-stub 1 = 4 (`swap_layouts.rs:38-57`, `layout.rs:923-933`) —
    so single-shell tabs re-fit BASE, while multi-pane tabs skip it and
    arrive already at "docked" (unconstrained swap tabs parse to
    `NoConstraint`, `kdl_layout_parser.rs:2052`).
  - **A visible floating pane shadows the name.** `TabInfo`'s swap name
    reports the FLOATING layer whenever floating panes are visible
    (`tab/mod.rs:1005-1017`), and every tab's floating list carries a
    birth "BASE" (`swap_layouts.rs:52-54`). During a bootstrap retrofit
    the floating actor itself keeps the tab reporting "BASE", one hop
    before the override lands.
  The steer therefore fires **by reported entry, toward the target**:
  from "BASE" docked is one forward step and undocked one deterministic
  backward wrap; from "docked" toward undocked one forward step (and the
  mirror backward); a report already *at* the target stands the press
  down — one more step would overshoot (observed live: the bootstrap tab
  landed one entry past docked, on the undocked sliver). While floating
  panes are visible the press stays armed. The damage flag is no part of
  the signature — a landed override was observed live still reporting the
  tab dirty. Because swap presses act on the client's *active* tab, the
  recorded press is abandoned if the tab closes or loses focus first —
  the regenerated set stays installed, and a later press steers by name.
  A foreign name likewise abandons the press (the override failed or
  raced); only a nameless report (the one-selectable-pane blind spot)
  keeps it waiting.
- **Fallback.** Any missing input — permissions not yet granted, no tab id,
  dump error/timeout, un-rebuildable dump (no tab node, chrome-only tab) —
  degrades to the v2 two-call cycle: the arrangement snap-folds, but the
  toggle still lands.
- **Tab ids vs positions.** `dump_session_layout_for_tab` and
  `get_focused_pane_info` speak the server's stable tab **id**
  (`screen.tabs` is keyed by `tab.id`; `active_tab_ids` stores ids), while
  `PaneManifest` keys and `TabInfo.position` are display **positions** —
  equal until any tab is closed or moved. The plugin records
  `TabInfo.tab_id` per position from `TabUpdate` and translates in both
  directions; a stale id fails safe (the dump returns no tab node → fallback).
- **Retrofit = the same rebuild, aimed at the active tab.** Floating
  residents and sidebar-less tabs run the identical dump → transform →
  override → deferred-steer machinery against the *active* tab (its server
  id translated from the TabUpdate states): the override's base spawns the
  rail while absorbing the tab's panes, and the post-override relayout plus
  the deferred press land the tab docked — single-shell tabs re-fit BASE
  (already the docked geometry), multi-pane tabs arrive at "docked"
  directly, and the steer only adds the step the relayout left missing —
  so the tab arrives docked with its splits intact. Chrome an earlier absorb ate (status-bar inside the stack,
  observed live) arrives in the dump and leaves repaired: the transform
  extracts chrome from wherever the dump seats it and re-emits canonical
  rows. When the rebuild cannot run — permission not granted, no tab id,
  dump or transform error — the retrofit **defers** (returns without
  acting): a capable instance with fresh `tab_states` docks the tab, or the
  next press does. The blind stacked-absorb fallback was removed
  (`absorb_retrofit` and `retrofit_layout_kdl` are deleted), because under
  the relaxed election it would blind-stack a tab another instance had
  already docked correctly, destroying the user's splits (arc entry below).
  The split-preserving base still keeps the `stacked { children }` main —
  the only retained-pane-correct shape, proven three times (do not change).

**Anomaly resolved (2026-07-04, v0.44.1 source trace).** The 2026-07-03
CLI no-op and the 2026-07-02 mistarget are one law: **a CLI
`override-layout --apply-only-to-active-tab` cannot be aimed.** A CLI
action runs as the *last client to have sent a Key message*
(`route.rs:2316-2332` → `get_last_active_client`, `lib.rs:511`, set only
by Key messages at `lib.rs:641-643`, cleared on that client's disconnect
at `lib.rs:547-549`), falling back to the CLI client's own id. The screen
then resolves `active_tab_ids[that client]` (`screen.rs:6883-6923` →
`get_active_tab_mut`, `screen.rs:2238-2246`). With no key-active client,
the CLI's own id has no active-tab entry: one screen-thread error line,
an empty tab list flows through the rest of the pipeline, exit 0, tab
untouched — the observed silent no-op. With one, the override lands on
*that client's* focused tab, which is what `list-clients` (not the dump's
`focus=true`) reports — the Tab #1-instead-of-#2 morning result. The
**plugin host-call path is different and sound**: `run_action` acts as
`env.client_id` (`zellij_exports.rs:1421-1447`), the connected client the
instance was loaded for (`wasm_bridge.rs:292-318`), so a plugin override —
from *any* tab's instance — targets the attached user's true focused tab.
Proven live in drill 7 (2026-07-04): two remote-election retrofits from a
background tab's rail installed rails on the user's focused tabs.

### Toggle v3.7–v3.12 — hardening under concurrency, load, and real dump shapes (validated live 2026-07-04..07, ztest)

Toggle v3 above is the mechanism; the entries below are what it took to make
it hold under rapid toggling, many tabs, and the layout dumps zellij actually
emits. All shipped at HEAD.

- **Relaxed retrofit election (v3.8).** The first design elected one instance —
  the session's lowest sidebar pane id — to retrofit a sidebar-less active tab.
  Under rapid toggling each instance's `tab_id → position` translation goes
  stale at a different rate, so instances disagree on which tab is active in
  the same instant: the elected instance often retrofitted a stale tab and
  aborted, while the instances that saw the bare tab correctly were barred — a
  brand-new tab could deadlock and never dock. The gate is dropped: **any
  instance that observes a sidebar-less active tab retrofits it.** Plugins
  process a pipe concurrently (one pinned server thread per instance), so
  several may retrofit the same tab on one press; three layers dedup them — the
  dump abort (below), the server re-seating a rail by `(url, config)` rather
  than spawning a second, and the idempotent transform. Where two tiled rails
  still race in, the **higher pane-id one is redundant and closes itself**,
  leaving the single lowest-id resident. Removing the election also cured its
  crashed-instance starvation (SPEC #34): a crashed sidebar persists in the
  manifest and the dump, so a retrofit into that tab aborts on the dump before
  a second rail is ever seated.

- **Dump-abort seeder dedup (v3.7).** A retrofit dumps the target tab fresh
  from the server before overriding; if any sidebar rail survives that dump
  (the server strips only the requesting plugin's own pane), the tab is already
  retrofitted and the override aborts. This dedups concurrent retrofits and
  discriminates "already ours" from "fresh split tab" — `swap_name` cannot,
  because a just-overridden tab momentarily reports "BASE", identical to a
  never-retrofitted split tab, so the authoritative signal is the dump, not the
  cached manifest.

- **Remote retrofit arms no steer.** The deferred steer is recorded only when
  the acting instance lives in the tab it rebuilt. A remote retrofit (an
  instance docking a tab it does not live in) cannot fire the steer —
  `previous/next_swap_layout` act on the client's active tab — and the
  override's own relayout lands that tab docked anyway, so its resident owns
  every later toggle.

- **Defer, never blind-absorb (v3.9).** Under the relaxed election, an instance
  whose `tab_states` is too stale to resolve the tab id, or whose
  dump/transform fails, must not fall through to a split-less stack override —
  that blind-stacks a tab another instance already docked correctly and
  destroys the user's splits (observed live: absorb fired 45× in one drill,
  several on tabs already reporting "docked"). It now **defers** instead;
  `absorb_retrofit` and `retrofit_layout_kdl` are deleted, `rail_pane_kdl`
  stays (used by `split_preserving_layout_kdl`).

- **Poll only the visible docked rail.** `refresh_statuses` forks a `ps`
  (`GetPaneRunningCommand`, 100ms server budget) per pane on a 2s timer; with
  one rail per tab, N tabs polling M panes saturated the PTY thread, busy panes
  deterministically timed out (drill 13: 337 timeouts in 2 min), and toggle
  dumps queued behind the storm. Only the active tab's **docked** rail is on
  screen, so the poll is gated on visibility — an undocked sliver and every
  background tab's rail poll nothing, collapsing ~N pollers to one. The gate
  reads `reported_active_tab`, set only in the `TabUpdate` handler from the
  server's authoritative `t.active` — **not** `active_tab`, which
  `perform_toggle` also overwrites via the stale `tab_id → position`
  translation (that dual writer left the gate uncertain and over-polling: drill
  14 still saw 266 timeouts). Failing panes back off (skipped for a growing run
  of timers — 0, 1, 3, 7, 15, then flat ~30s, reset on success), because a
  timeout `Err` is shaped like not-found and a naive poll otherwise retries a
  stuck pane forever. A skipped poll shows the pane's previous status, never a
  blank, and heals on the next `TabUpdate`.

- **Retired debounce and launch behavior (v3.4).** A press for a tab whose
  JIT pipeline is still in flight, or whose steer fired < 600ms ago, is
  swallowed — the pipeline's visible collapse lags the press, so a quick second
  press otherwise reads as an instant re-toggle. And a toggle pipe that reaches
  a just-launched instance before its first `PaneUpdate` (the keybind's
  launch-if-missing races its own pipe) is parked, not dropped, and consumed
  exactly once on the first manifest that names the instance with its active
  tab known. The current fail-closed route intentionally drops that press.

- **Single-line chrome extraction (v3.11).** zellij's layout dump serializes a
  chrome or rail pane with its plugin child inline on one line —
  `pane size=1 borderless=true { plugin location="zellij:tab-bar" }` — which
  the chrome extractor previously treated as an opaque leaf and left in the
  user region. On mixed vertical/horizontal split tabs that baked the tab-bar
  mid-layout and left a single-line rail un-dropped as a stray second rail. The
  extractor now reads the plugin location off a single line and classifies it
  exactly like the multi-line path, so top-level chrome, nested chrome, and
  stray rails are all hoisted or dropped correctly.

- **Trace behind a debug gate (v3.6).** Every toggle-path decision emits one
  greppable `zaphod-trace[<id>]:` line to stderr (which zellij routes to
  `zellij.log`, never the pane) when a non-empty `debug` config key is set;
  production behavior is byte-identical when the gate is off.

#### The split-preserving fidelity ceiling (accepted, 2026-07-07)

One structural loss survives every fix above and is **not** a plugin bug: a
deeply-nested percentage region flattens when the rail width flips. Live A/B on
a mixed 3-pane v/h tab (drove `next_swap_layout`, dumped each state; region =
right of the size-28/1 rail):

| State | Region dump | Structure |
|---|---|---|
| docked (rail=28) | `pane 50% { pane 50%; pane 50% }` + `pane 50%` | nested — a column of two rows beside a column |
| undocked (rail=1) | `pane 50%` + `pane 25%` + `pane 25%` | flat — three side-by-side columns; nesting gone, sizes recomputed |

The split-preserving transform is innocent — it emits a **byte-identical**
nested region in both swaps, differing only by rail width (verified offline).
The loss is entirely in zellij's swap re-seat: applying a swap resolves the
layout to absolute leaf geometry for the current free space
(`LayoutApplier::apply_tiled_panes_layout_to_existing_panes → flatten_layout`,
`zellij-server/src/tab/layout_applier.rs:160,357`), not by re-applying the
tree; when the free space changes (the 28→1 rail flip widens the region)
`position_panes_in_space`'s percentage-constraint solve can fail, and
`flatten_layout`'s `.or_else` fallback then re-positions **ignoring the
percentage sizes** — zellij's own comment there reads "a hack around some
issues with the constraint system that should be addressed in a systemic
manner" (`layout_applier.rs:370-388`). No template shape survives it
(`flatten_layout` discards the tree, and the failing step is the percentage
solve, not the structure). **Panes and content always survive — only the split
nesting is lost.** Still present on zellij main (post-0.44.3), unfixed in any
release. Accepted as the ceiling of split preservation; upstream family: zellij
#1825, #2829, #1758, #4647. (SPEC landmine #35.)

### Permission gating (zellij v0.44.1 source, verified)

`override_layout` is gated by **`ChangeApplicationState`**
(`zellij-server/src/plugins/zellij_exports.rs:5289`); `next_swap_layout`
likewise (`:5243`). Both are fire-and-forget shims: **a denied call is a silent
no-op** — no panic, no error reaches the plugin, so the plugin must hold the
grant before toggling; there is no failure signal to react to. With the spawn
machinery deleted, `OpenTerminalsOrPlugins` has no consumer and is dropped; the
minimal grant set is `ReadApplicationState` + `ChangeApplicationState` +
`ReadPaneContents`.

### What the rework deleted — and what it kept

The shipped prototype architecture removed the summon/spawn machinery and the degraded
absorb path. The former leader-election state was **kept and repurposed** for
the relaxed election, not deleted — a distinction that matters because an
earlier plan slated it for removal.

**Deleted:**

- The `ToggleAction::SpawnInActive` / `::HideSelf` / `::ShowHere` variants —
  their `pipe()` / `ensure_visible_in_active_tab` arms and `decide_toggle`
  production sites — and `toggle_action_requests_render` with them.
- The summon machinery: `float_as_rail`, `rail_coordinates`, the crate's
  `float_multiple_panes` / `change_floating_panes_coordinates` /
  `open_plugin_pane_floating` calls, `RAIL_WIDTH`, `rail_mode`,
  `rail_positioned`, and render's first-float rail snap.
- The out-of-set `hide_self` / `show_self` sites (the pre-render summon blocks
  in `pipe()` and the header-body click-to-hide) and the
  `unblock_cli_pipe_input` call — it needed the never-requested `ReadCliPipes`
  grant and CLI pipes auto-unblock once `pipe()` returns anyway.
- `absorb_retrofit` and `retrofit_layout_kdl` (removed with the
  defer-not-absorb change, b8b4231); `rail_pane_kdl` stays, still used by
  `split_preserving_layout_kdl`.
- The `OpenTerminalsOrPlugins` grant and the `can_spawn` gate — no consumer
  once the spawn path is gone.

**Kept and repurposed** (the relaxed election reuses the old leader-election
satellites rather than deleting them):

- `sidebar_instances()` and the `instances` state — the lowest-id leader gate
  was replaced by any-instance-retrofits plus the `is_redundant_tiled_sidebar`
  cleanup, both of which read the session's sidebar instances.
- `own_url` — feeds the retrofit override, which re-seats the tab's rail by
  `(url, config)`.
- `decide_toggle` — stays as the toggle head, with a shrunken signature.

### Hazard for layouts that ship the sidebar (verified live)

A fresh session whose layout **focuses** the sidebar pane crashes the whole
zellij server: the plugin's focus-handback (`own.is_focused && !nav_mode` →
`focus_previous_pane()`) fires in the session-birth window and the server
panics at `zellij-server/src/panes/tiled_panes/mod.rs:1837`
(`get_active_pane_id().unwrap()` on `None`). Mid-session the call is safe.
Layouts must not focus the sidebar; the plugin-side guard lands with the toggle
rework, and an upstream zellij report is planned.

## Prototype / worktree info

- **Repo:** `/Users/clkao/git/zaphod` (crate `zellij-sidebar`).
- **Working branch:** `wip/override-layout-docking`, based at `7c0d2a6`
  ("Fix reentrant sidebar toggle routing"). It carries the whole toggle
  rework — 21 commits (`d6ad89b`…`bce89a4`) past `main`; HEAD is `bce89a4`.
- **Untracked:** `.safehouse` (safehouse config; `add-dirs=~/.config/zellij`,
  `add-dirs=~/git/spacedock-research`).
- **Git worktrees:**
  - `/Users/clkao/git/zaphod` → `bce89a4` `[wip/override-layout-docking]` (primary).
  - `/private/tmp/zaphod-review-4ddbf148` → `4ddbf14` (detached HEAD, **prunable** — stale
    review worktree; `git worktree prune` to remove).
- **Build:** `./build.sh` →
  `target/wasm32-wasip1/release/zellij-sidebar.wasm`. Uses `rustup`'s rustc
  (`RUSTC="$(rustup which rustc)" cargo build --release --target wasm32-wasip1`) because
  homebrew rust shadows rustup and lacks the `wasm32-wasip1` std. The artifact
  is rebuilt across the arc (latest 2026-07-07); the only rollout item still
  pending is retiring the old-wasm **instances** left running in live sessions
  (a live-session step, not a source rebuild).
- **Versions:** zellij CLI 0.44.1; `zellij-tile` locked 0.44.3 (current release 0.44.3,
  bug-fix-only).
- **Source changed:** `src/main.rs` carries the shipped rework — the summon /
  hide / show / spawn machinery removed, the split-preserving retrofit and
  relaxed election, the poll-visibility gate, and the chrome extraction;
  `src/agent.rs` (agent awareness) is unaffected.

## Install provenance and candidate testing

Zellij identifies a plugin instance by URL plus configuration. Zaphod's
identity boundary is therefore the canonical WASM `file:` URL together with
`rail "1"`; independently valid layout and keybind files can still name two
different instances.

`install.sh` is the only global installer. It resolves Git's physical primary
checkout, refuses linked worktrees before its first write, and validates every
Zaphod `MessagePlugin` in the effective `config.kdl`. It renders the production
layout to a destination-local temporary file, validates the config, parses the
layout in a disposable Zellij 0.44.3 session, renames it atomically, and repeats
the identity check. A failed postflight restores the previous bytes or removes
a new layout. The installer diagnoses keybind mismatches; it never edits them.

Candidate testing uses `./tests/zellij-tmux-smoke-test.sh` from the candidate
worktree. It starts one Zellij client inside a dedicated tmux server with short
isolated config, data, and socket roots, then runs the real direct-entry script
in that session. Native `list-panes`, `list-tabs`, and `dump-layout` verify one
candidate rail at the returned stable tab ID. Literal tmux keys prove managed
`Alt /` behavior and foreign-tab inertness. The isolated profile's fixed
`Alt Shift z` route remains byte-identical and is not selected-checkout
evidence. Normal exit, TERM, INT, or HUP deletes the session, kills the tmux
server, and removes the temporary root. Cleanup compares the existence and
SHA-256 of both isolated and standing config/layout files; it reports mutation
and never overwrites concurrent changes by trying to restore them.

Use live Zellij state as the oracle. `action list-panes --json -a -g -t` proves
pane IDs, counts, kinds, geometry, and cwd. `action dump-layout` proves the URL,
configuration, and chrome that the server loaded. Generated KDL or a source
grep cannot prove resident identity. Bracket every live drill with global
config/layout hashes, and run unmerged j5 or later candidates only through the
tmux smoke harness. Never run a linked worktree's `install.sh`.
