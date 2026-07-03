# Docking via layouts — findings and proposed approach

> Investigation date: 2026-06-20 · adopted architecture validated 2026-07-02
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
  the adopted architecture (below).
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
that lack the layout (SPEC #12). The adopted architecture (below) supersedes it: the
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
the time for tabs without a docked instance; the adopted architecture (below)
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

## Adopted architecture (validated live, 2026-07-02)

The end state, mirroring yazelix's model:

**The sidebar pane exists in every toggled tab's layout, permanently.** The
default layout is chrome-only; a tab's first toggle retrofits the sidebar in.
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
2. **Retrofit (option a)** — for a live tab without the swap set, a one-time
   `override_layout(LayoutInfo::Stringified(kdl), retain_terminals=true,
   retain_plugins=true, apply_only_to_active_tab=true)` whose KDL contains the
   tab-bar/status-bar chrome, a stacked main that absorbs the existing panes,
   the `rail "1"` sidebar slot, and **both** `swap_tiled_layout` sections. The
   override installs the swap set on that tab; every subsequent toggle is pure
   swap cycling. (Confirmed viable — no per-toggle re-override needed.)

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

**Retrofit arm — VERIFIED.** With the session's sole sidebar instance
floating in another tab, `Alt /` on a sidebar-less tab ran the cross-tab
election (lowest pane id acted), the override hit the *active* tab rather
than the actor's, the unnamed `tab` node preserved the tab's name, and
post-retrofit toggling cycled cleanly.

**Bootstrap gap — CONFIRMED, fix shipped with toggle v2.** In a fresh
chrome-only session, the `Alt /` keybind's launch-if-missing spawned a
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
  the *old* swap set. The press is therefore recorded and fired on the
  `TabUpdate` that reports the override's signature: position 0 ("BASE").
  The damage flag is no part of the signature — a landed override was
  observed live still reporting the tab dirty, and a press deferred on
  "BASE and clean" never fired. Because swap presses act on the client's
  *active* tab, the recorded press is abandoned if the tab closes or loses
  focus first — the regenerated set stays installed, and a later press
  steers from BASE. A `TabUpdate` reporting any *other* name likewise
  abandons the press (the override failed or raced) rather than keeping it
  armed forever; only a nameless report (the one-selectable-pane blind
  spot) keeps it waiting.
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
  rail while absorbing the tab's panes, and the deferred press steers
  forward from BASE to docked — a plain position 0→1 increment, never the
  flaky next-past-end zone — so the tab arrives docked with its splits
  intact. Chrome an earlier absorb ate (status-bar inside the stack,
  observed live) arrives in the dump and leaves repaired: the transform
  extracts chrome from wherever the dump seats it and re-emits canonical
  rows. When the rebuild cannot run — permission not granted, no tab id,
  dump or transform error — the v2 absorb override runs instead, with no
  steer: its base lands directly on the docked geometry (the arrangement
  stacks, the toggle still docks). The absorb KDL keeps the
  `stacked { children }` main (proven three times — do not change).

**Open anomaly (unresolved, 2026-07-03).** CLI
`override-layout --apply-only-to-active-tab` silently **no-oped twice** on
a chrome-only-template tab: client verified attached and focused
(`list-clients`), KDL parse-clean (accepted by `new-tab`), exit 0, zero
server log lines, tab byte-identical after. The same CLI form worked twice
on 2026-07-02 (different tabs, different KDLs), and the **plugin host-call
override has always worked** (the shipped v2 retrofit). Difference not
understood — candidates: swap-section content (concrete percent panes,
`focus=true` in a swap), tab born from a chrome-only template, session
state. v3's regeneration uses the plugin host call, so it is not known to
be affected — but its live validation must watch for this failure class.
Related ground-truth caveat: a dump's `focus=true` does **not** identify
the acting client's focused tab (the first override that morning hit Tab #1
while the dump claimed Tab #2 focused); `list-clients`' ZELLIJ_PANE_ID is
the authority on where `--apply-only-to-active-tab` lands.

### Permission gating (zellij v0.44.1 source, verified)

`override_layout` is gated by **`ChangeApplicationState`**
(`zellij-server/src/plugins/zellij_exports.rs:5289`); `next_swap_layout`
likewise (`:5243`). Both are fire-and-forget shims: **a denied call is a silent
no-op** — no panic, no error reaches the plugin, so the plugin must hold the
grant before toggling; there is no failure signal to react to. With the spawn
machinery deleted, `OpenTerminalsOrPlugins` has no consumer and is dropped; the
minimal grant set is `ReadApplicationState` + `ChangeApplicationState` +
`ReadPaneContents`.

### Approved deletion list

Mapped with file:line citations at HEAD `95d8eda` (baseline: 37/37 tests green;
`src/agent.rs` contains none of the symbols):

- `ToggleAction::SpawnInActive` / `::HideSelf` / `::ShowHere` — the variants,
  their `pipe()` / `ensure_visible_in_active_tab` arms, and their
  `decide_toggle` production sites. `toggle_action_requests_render` collapses
  with them.
- Leader election inside `decide_toggle` (`src/main.rs:424-440`) and its
  satellite state: `instances`, `sidebar_instances()`, `own_url`
  (`permissions_granted` becomes write-only). `decide_toggle` itself stays —
  the SwapLayout head — with a shrunken signature.
- The summon machinery: `float_as_rail`, `rail_coordinates`, the crate's only
  `float_multiple_panes` / `change_floating_panes_coordinates` calls,
  `RAIL_WIDTH`, `rail_mode`, `rail_positioned`, and render's first-float rail
  snap (`src/main.rs:257-260`).
- The out-of-set `hide_self`/`show_self` sites: the pre-render summon blocks in
  `pipe()` (`src/main.rs:170-176`, `:184-195`) and the header-body
  click-to-hide (`:368-369`) — `handle_click` is redesigned, and the navigate
  flow through `ensure_visible_in_active_tab` is rethought as that fn
  collapses.
- The `unblock_cli_pipe_input` call (`src/main.rs:166-168`): it needs the
  never-requested `ReadCliPipes` grant, is silently denied today, and CLI pipes
  terminate anyway via the server's auto-unblock once `pipe()` returns.
- `OpenTerminalsOrPlugins` from the permission request list (`src/main.rs:252`)
  and the `can_spawn` gate.
- Tests: four spawn/leader tests deleted, two hide/show tests deleted or
  rewritten, one swap-layout test updated to the shrunken signature.

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
  ("Fix reentrant sidebar toggle routing"). At time of writing the branch has **no commits
  yet** — it is identical to `main`; all work so far is investigation, no source edits.
- **Untracked:** `.safehouse` (safehouse config; `add-dirs=~/.config/zellij`,
  `add-dirs=~/git/spacedock-research`).
- **Git worktrees:**
  - `/Users/clkao/git/zaphod` → `7c0d2a6` `[wip/override-layout-docking]` (primary).
  - `/private/tmp/zaphod-review-4ddbf148` → `4ddbf14` (detached HEAD, **prunable** — stale
    review worktree; `git worktree prune` to remove).
- **Build:** `./build.sh` →
  `target/wasm32-wasip1/release/zellij-sidebar.wasm`. Uses `rustup`'s rustc
  (`RUSTC="$(rustup which rustc)" cargo build --release --target wasm32-wasip1`) because
  homebrew rust shadows rustup and lacks the `wasm32-wasip1` std. Current artifact built
  2026-06-10 (predates this investigation; no rebuild needed yet — no source changed).
- **Versions:** zellij CLI 0.44.1; `zellij-tile` locked 0.44.3 (current release 0.44.3,
  bug-fix-only).
- **Source under change:** `src/main.rs` — the adopted-architecture deletion set
  (`float_as_rail`, the spawn/hide/show `ToggleAction` arms, leader election in
  `decide_toggle`, `ensure_visible_in_active_tab`, the `handle_click` redesign);
  `src/agent.rs` (agent awareness) is unaffected.
