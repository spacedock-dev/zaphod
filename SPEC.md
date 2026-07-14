# Zaphod — zellij pane-switcher sidebar

> **Historical prototype record.** This file documents the shipped Zellij WASM
> sidebar and its numbered technical landmines. It is not the product
> architecture. The evergreen direction uses one Zaphod-managed tab or window,
> leaves foreign views unchanged, and is defined in
> [`docs/zaphod-workspace-architecture.md`](docs/zaphod-workspace-architecture.md).

Spec and from-scratch learnings, distilled from the v1 prototype campaign
(zellij 0.44.1, 2026-06-10).

## What it is

A per-tab sidebar that answers "what is running in this tab and what does it
want from me?" — built for stacked-pane workflows where many Claude/codex
agents run side by side.

The rail also renders agent-session and pending-gate rows fed over the
`agent-event` pipe by grout — protocol and binding rules in
`docs/archive/plan-agent-rail-prototype-2026-07-07.md` (decisions 1-3): two
typed JSON kinds, cwd binding
in the plugin via `get_pane_cwd`, unbound rendered as unbound, never guessed.

### Validated UX (what the prototype proved out with real use)

- **Left rail, ~28–30 cols**, listing the current tab's terminal panes,
  top-to-bottom.
- Per pane row (two lines):
  - markers: yellow `●` focused pane, red `●` agent working (viewport contains
    `esc to interrupt`), cyan title for agent panes (`✳` in title — Claude
    Code/codex set this)
  - dim second line: the pane's last non-empty terminal line (what the agent
    is asking/doing), polled every 2s via `get_pane_scrollback`

  This prototype record remains historical. The shipped periodic path retired
  the scrollback call because Zellij 0.44.3 can synchronously hold that export for five seconds. The sidebar now preserves stale status or shows a fresh unresolved pane as unknown.
- **Click a row → focus that pane.** The sidebar itself is unfocusable
  (`set_selectable(false)`, the tab-bar mechanism) so it never steals focus
  and clicks work without focus-then-click.
- **`Alt /`** toggles the sidebar in the active tab; per-tab instances toggle
  independently.
- **`Alt .`** keyboard nav: the pane becomes focusable + focused, `j/k`/arrows
  move a highlight, `Enter` jumps to the selected pane, `Esc` restores the
  previous focus. Focusability is dropped on exit.
- Header: click body → hide; `⇄` at right edge → toggle floating rail vs
  docked tile.
- Two presentation modes:
  - **floating pinned rail** (x=0, y=1, fixed width, ~97% height, pinned):
    the only runtime-exact placement; overlays rather than reserving space
  - **docked tile**: reserves space (what the user actually prefers), but
    only achievable via layouts — `default_tab_template` in the default
    layout for new sessions, `new-tab --layout` for on-demand tabs
- User verdicts from iteration: per-tab context is non-negotiable; floating
  is acceptable for summon→glance→jump but "not quite right" as a persistent
  sidebar because it does not push existing panes aside; full height matters;
  focus theft is immediately noticed.

### Feature roadmap (not yet built)

- Richer agent states: working / waiting-for-input / idle, codex busy markers
- Scrolling when the pane list exceeds the rail height (currently clips)
- A tabs section (clickable, vertical-tabs style) above the pane list
- Config: width, poll interval, status lines on/off, busy-marker patterns

## Platform learnings (zellij 0.44.1) — the landmine map

Every entry below cost real debugging time. A from-scratch v2 should treat
these as laws.

### Plugin lifecycle

1. **The compiled-plugin cache is path-keyed and in-memory** — first compile
   wins for the entire session; rebuilds at the same path are silently
   ignored (`load_module_from_memory` checks no mtime/hash). Dev loop
   requires `skip_cache true` on the launching keybind, or a changed
   configuration identity. This masked several fixes and burned hours.
2. **Launch/pipe identity = URL + configuration.** A distinct config key
   (`rail "1"`) creates a fresh identity — the escape hatch for orphaned
   instances.
3. **Pipe-launched plugins without explicit `floating` land in the
   background**: pane-less instances that swallow all future pipes and cannot
   materialize a pane (`show_self` is a no-op without a pane).
   `start-or-reload-plugin` also creates background instances. Zombies are
   only killable via plugin-manager or session restart. The `Alt /` toggle
   pipe must reach the resident layout instance: its keybind configuration
   must exactly match the layout plugin's (`rail "1"`), or the pipe launches
   a fresh instance instead — this zombie class, or a stray floating one.
4. **Never call response-reading/blocking shim functions inside `pipe()` or
   `load()`** — `show_floating_panes` deadlocked the launch (plugin blocks on
   server response, server waits on handler); `open_plugin_pane_floating`
   crashed the instance outright (`wasm unreachable`). Side effects that need
   responses must be deferred to normal event handlers or avoided.
5. **CLI pipe sources stay blocked** while any plugin is still processing the
   pipe or holds an explicit block; the server auto-unblocks once every
   plugin's `pipe()` has returned, so a synchronous `pipe()` needs no explicit
   `unblock_cli_pipe_input`. The explicit call requires the `ReadCliPipes`
   grant — without it, it is silently denied (verified against v0.44.1 server
   source: `pipes.rs` NoChange bookkeeping + `zellij_exports.rs:5297`).
   A CLI pipe stays blocked until every directed instance's `pipe()` has
   returned: each instance is marked at dispatch (`wasm_bridge.rs:1174-1184`),
   and an explicit `unblock_cli_pipe_input` from a sibling cannot release it —
   the unblock only clears the explicit-block flag and the caller's own
   membership; release requires the processing set empty (`pipes.rs:107-127`).
   The only cross-instance release is plugin unload (`wasm_bridge.rs:602`).
   Explicit unblock is therefore useless against a wedged sibling (verified
   v0.44.1; corroborated by the 2026-07-07 receipt-then-hang spike).

### Permissions

6. `GetPaneScrollback` requires **`ReadPaneContents`** — without it every
   call is denied, visible only as 2s-interval log spam.
7. **`request_permission` during `load()` races pane registration**: the
   request gets parked in `pending_events_waiting_for_client` and replays
   only when a *new client attaches* — the prompt simply never appears in a
   long-lived session. Request after the pane demonstrably exists (first
   render worked once launches were visible).
8. **The permission prompt needs a focusable pane** (`y` is pressed inside
   it). Defer `set_selectable(false)` until after the grant arrives.
9. Permission grants cache in `<cache-dir>/permissions.kdl`, keyed by plugin
   location string. Pre-granting by writing this file works (useful for
   headless test benches). On macOS the cache dir is
   `~/Library/Caches/org.Zellij-Contributors.Zellij/` (not Application
   Support).

### Placement

10. **Runtime tiled placement is unwinnable** in 0.44:
    - `move_pane` (CLI and plugin API) is swap-based — a bottom-spawned
      full-width pane has no left neighbor, so "move left" no-ops
    - `new-pane --plugin` forbids `--direction`
    - `override-layout` is destructive: it seated panes at 1-row heights and
      lost terminals despite `--retain-existing-terminal-panes`. Never run it
      against a tab that matters.

    Revised (validated live — see `docs/docking-approach.md`):
    `override-layout` is a full-tab **replacement**, not a rail-adder. Applied
    with a complete KDL (tab-bar/status-bar chrome + a stacked main that
    absorbs existing panes) and the retain flags, it reserves tiled space
    cleanly, keeps existing terminals, and installs any embedded
    `swap_tiled_layout` set on the tab. The destructive result above came from
    incomplete KDL — empty slots spawn shells, omitted chrome is dropped.
11. **Layouts are the only reliable tiled placement**: `default_tab_template`
    in the default layout (new sessions), `new-tab --layout` (on demand).
    Note: `default_tab_template` in `config.kdl` is *silently ignored* — it
    is a layout-file construct only. A layout can also be applied to a live
    tab at runtime — inline KDL via `override_layout` (see #10's revision).
12. **Floating + `change_floating_panes_coordinates` is the only runtime-exact
    placement.** `pinned` panes survive the user's floating-layer toggles.
13. **Tabs with `hide_floating_panes` make floating spawns invisible — and an
    invisible pane never renders**, so any render-gated logic (positioning,
    permission requests) silently never runs. Summon paths must not depend on
    rendering.
14. `embed_multiple_panes` lets a plugin dock itself (zellij picks the slot);
    a resize-hysteresis loop (`resize_pane_with_id`, shrink-only, no-change
    guard, give-up cap) keeps a tiled sidebar narrow across layout reflows.
15. **Tiled resizes and float-transitions trigger auto-layout reflows** that
    can restack the user's entire tab (`stacked_resize` + swap layouts +
    ≥5 panes). Gate every such command on manifest-confirmed pane state —
    pre-manifest defaults (`own_floating=false`) caused exactly this.

### Focus

16. **`show_self` always focuses the pane** — and for a pane in another tab,
    it yanks the user's view to that tab. Cross-tab "follow me" via
    `show_self`/`break_panes_to_tab_with_index` is therefore unusable. The
    per-tab sibling-spawn model (`open_plugin_pane_floating`) that replaced
    it is retired too. The shipped prototype kept a sidebar pane in each tab
    it retrofitted and toggled by cycling swap layouts
    (`docs/docking-approach.md`), so its steady-state toggle showed, hid, and
    spawned nothing. The evergreen product instead creates or focuses one
    managed view and never retrofits a foreign tab.
17. **`set_selectable(false)` does not evict already-resident focus** — the
    pane arrives focused and stays "focused" visually. A manifest-driven
    bounce (own pane `is_focused` outside nav mode → `focus_previous_pane`)
    is self-correcting; blind bounces after every show degenerate into
    focus-cycling when state drifts. Caveat (verified live, 0.44.1): if a
    fresh session's *layout* focuses the plugin pane, the bounce fires in the
    session-birth window and panics the whole server —
    `get_active_pane_id().unwrap()` on `None`
    (`zellij-server/src/panes/tiled_panes/mod.rs:1837`). Guard the handback
    until the manifest shows another focusable pane.

### State

18. **Internal flags drift; the PaneManifest is ground truth.** Derive
    hidden-ness (`is_suppressed`), floating-ness (`is_floating`), own tab,
    own URL, and the instance list from every `PaneUpdate`. The prototype's
    sticky `hidden` flag desynced and turned toggles into no-ops.
19. Suppressed panes report `is_floating=false`; never use live manifest
    state to decide how to re-show a hidden pane (track intent — rail vs
    docked — separately).
20. The mouse API delivers `LeftClick(line, col)` pane-relative; clicks on an
    unfocusable pane are delivered without focusing it (tab-bar mechanism).
    Plugin Mouse enum is narrow: Left/Right click, Hold, Release, Hover,
    Scroll — no middle click, no per-button release.

### Diagnostics & dev loop

21. The server log is the oracle:
    `$TMPDIR/zellij-<uid>/zellij-log/zellij.log` (all sessions share it).
    Permission denials, plugin loads/compiles ("Loaded plugin ..." = real
    compile; absence = cache hit), parked permission requests ("PluginId not
    found - caching request"), and wasm crashes all appear here.
22. Headless bench: `zellij attach NAME --create-background` +
    `ZELLIJ_SESSION_NAME=NAME zellij action ...` (pi-zellij pattern). Caveat:
    a server spawned from a sandboxed process inherits the sandbox and breaks
    on cache/data dirs; redirecting `HOME` works but cold plugin caches and
    wasmtime locks make first loads slow/wedgy.
23. Useful CLI surface (0.44.1): `list-panes -j -t -s -g -a`,
    `focus-pane-id`, `move-pane/close-pane --pane-id`, `stack-panes`,
    `query-tab-names`, `dump-screen --pane-id` (terminals only; plugins dump
    empty), `subscribe` (streaming pane content), `pipe` (blocks).

### Swap layouts & layout application

24. **Swap cycling has a damage latch.** Any manual split, user resize,
    terminal-window resize, or added tiled pane sets `is_tiled_damaged`; the
    next `next_swap_layout` then only re-applies (snap-folds) the current
    template **without advancing** (v0.44.1 `swap_layouts.rs:242-251`; damage
    sites `tab/mod.rs:2361`, `:2428`, `:3396`, `:3425`, `:5370`).
    Deterministic toggling must read `TabInfo.is_swap_layout_dirty` and issue
    two calls on a dirty tab — but zellij reports `(None, false)` for a tab
    with at most one selectable tiled pane (`tab/mod.rs:1005-1018`), so
    damage there is invisible. (The two-call answer is the fallback; the
    primary dirty-tab path regenerates the swap set from a dump — see #28-31
    and `docs/docking-approach.md`, Toggle v3.)
25. **Every tab gets a hidden BASE swap layout**: `set_base_layout` inserts
    the tab's birth layout at swap position 0, named "BASE", constrained
    `ExactPanes(birth pane count)` (`swap_layouts.rs:38-57`) — whether or not
    the layout carries a swap set. The first press on a fresh tab is often
    visually silent (BASE → an identical-geometry first swap), BASE stops
    fitting once the pane count changes, and the effective cycle length
    varies at runtime. Corollary: `TabInfo.active_swap_layout_name` reports
    "BASE" on tabs *without* any swap set too, so it cannot detect "this tab
    lacks my swap set".
26. **`dump-layout` emits a resurrection format, not a reapplication
    format**: a concrete `pane` node means "spawn a new pane here"; retained
    panes re-seat only into `children` insertion points (exact run-match
    first — `layout_applier.rs:160-221`). Re-applying a tab's own dump via
    `override-layout` duplicates every terminal (verified live, twice). An
    override KDL must absorb existing panes via `pane stacked=true
    { children }`.
27. **A custom default layout must carry an explicit `tab { pane }`**: a
    template-only layout (no `tab` node), or a bare `tab` whose template
    holds `children` inside a nested split, births zero terminals and the
    session exits immediately ("Bye from Zellij!") — both observed live.
28. **The base layout must carry exactly the same chrome as the swaps.** A
    bare `tab { pane }` base under chrome-carrying swaps wedged a tab
    permanently (observed live): fold-to-BASE re-seated the shells into the
    chrome plugins' 1-row borderless slots, destroying the
    tab-bar/status-bar panes — after which every swap was unfittable forever,
    because swaps never spawn and the chrome plugin nodes had no surviving
    panes to match. Layouts generated at runtime must carry exactly one
    canonical chrome node per chrome pane the tab actually has — extracted
    from wherever the dump seats it, because absorb can bake chrome into
    the user region (observed live: tab-bar inside a quadrant, inside a
    stack), and a baked copy carried verbatim mangles the tab permanently.
29. **Steer swap layouts by name; never blind-cycle.** `next` past the last
    entry resets the position to 0 *without applying* (live: next-spam stuck
    around the list end), while `previous` from 0 wraps deterministically to
    the last entry (`swap_layouts.rs`, `swap_tiled_panes` progress macro).
    Read `TabInfo.active_swap_layout_name` and issue one deliberate
    next/previous per press; with [BASE, docked, undocked] installed, every
    docked↔undocked and BASE→either move avoids the next-past-end zone.
30. **A dump's `focus=true` is not the acting client's focus.**
    `dump-layout` marked a tab focused while the attached client sat on
    another (observed live: an `--apply-only-to-active-tab` override landed
    on the *other* tab). `list-clients`' ZELLIJ_PANE_ID is the ground truth
    for which tab "active" means. Root law (v0.44.1 source, resolves the
    2026-07-03 CLI no-op): "active tab" is `active_tab_ids[acting client]`
    (`screen.rs:6883-6923`), and the acting client differs by caller — a
    **CLI action** runs as the last client to have sent a *Key* message
    (`route.rs:2316-2332`, `lib.rs:511`), falling back to the CLI's own
    unregistered id (one screen error line, empty apply, exit 0 — a silent
    no-op); a **plugin host call** runs as `env.client_id`, the connected
    client the instance was loaded for (`zellij_exports.rs:1421-1447`,
    `wasm_bridge.rs:292-318`), so plugin overrides from any tab's instance
    land on the attached user's true focused tab (proven live: two
    remote-election retrofits, 2026-07-04).
31. **Fold-to-BASE ignores pane-count fit and is destructive.** The
    dirty-tab first press re-applies the current template regardless of how
    many panes fit it (live: three panes crammed into a one-slot base); it
    folds to the *template*, not to the current arrangement.
32. **Tab ids and tab positions are different index spaces.**
    `PaneManifest` keys and `TabInfo.position` are display positions;
    `dump_session_layout_for_tab` and `get_focused_pane_info` speak the
    server's stable tab id (`screen.tabs` is keyed by `tab.id`,
    `active_tab_ids` stores ids — v0.44.1 source). They match until any tab
    is closed or moved. `TabInfo.tab_id` carries the id; translate at every
    boundary.
33. **`override_layout` and swap presses race when fired back-to-back.**
    The override dispatches on a spawned server thread
    (`run_action`, `zellij_exports.rs:1421`) while
    `next/previous_swap_layout` route synchronously, so an immediate press
    can cycle the *old* swap set. Defer the press to a later `TabUpdate` —
    but **a landed override does not report "BASE" unconditionally**:
    `tab.override_layout` relayouts immediately after installing the set
    (`tab/mod.rs:978-981`), advancing the tab to the first *fitting* entry.
    BASE's constraint is `ExactPanes(base template leaf count)` (4 for the
    chrome+rail+children absorb base), so single-shell tabs re-fit BASE
    while multi-pane tabs arrive already at "docked". And **while floating
    panes are visible the reported name speaks for the floating layer**
    (`tab/mod.rs:1005-1017`), whose birth entry is also "BASE" — during a
    bootstrap retrofit the floating actor itself keeps the tab reporting
    "BASE" before the override lands, and firing on it lands the tab one
    entry past the target (observed live: undocked sliver instead of
    docked). Resolve the deferred press by reported entry, toward the
    recorded target; stand down when the report *is* the target; stay
    armed while floating panes are visible. The damage flag is no part of
    the signature — a landed override was observed live still reporting
    the tab dirty.

    The 2026-07-03 CLI no-op is resolved — see #30: a CLI override cannot
    be aimed; it acts as the last key-active client, or silently applies
    nothing under the CLI's own id.
34. **A crashed instance stays in the `PaneManifest` and can eat an elected
    role forever.** `decide_toggle`'s session-wide election for a
    sidebar-less tab picks the lowest pane id across every
    `SidebarInstance` the manifest reports (`src/main.rs`, the
    `instances.iter().all(|i| i.pane_id >= own_pane_id)` branch); a wasm
    panic halts the instance's event loop but does not remove its pane from
    the manifest, so a crashed instance holding the session's lowest pane id
    permanently wins every election while never acting on it. Observed
    live: one tab's tiled rail crashed, and every sidebar-less tab across
    the session went dead to the toggle — both the keybind and a clean CLI
    pipe — while tabs that already had a live resident were unaffected
    (they short-circuit before reaching the election). No event or manifest
    field distinguishes "crashed" from "alive but momentarily quiet" —
    there is no plugin-side liveness signal, only the pane's continued
    presence — so `decide_toggle` cannot detect this case from its current
    inputs without inventing a heuristic (e.g. staleness timers) fragile
    enough to misfire on a merely slow instance. This is one instance of a
    broader upstream limitation class: nothing signals plugin crash/reload
    to other plugins or to the manifest. Recovery is manual: close the
    crashed pane (or restart the session) to drop it from the manifest and
    let the next-lowest live instance win the election.

    **v3.8 relaxation.** The lowest-id election above was removed: any
    instance that observes a sidebar-less active tab now retrofits it, so a
    crashed instance can no longer starve the role — the dead-tab symptom
    above is cured, because a live instance docks the tab regardless of a
    crashed pane's id. Concurrent retrofits (instances disagree on the active
    tab under pipe-flood staleness, so several may act on one press) are
    deduped by the JIT dump abort, and a redundant tiled rail — the higher
    pane-id sidebar of two sharing the tab — closes itself, leaving the
    lowest-id resident. That cleanup could in
    principle close a *live* rail if a crashed sidebar's dead pane lingered
    with a lower id, but the window is unreachable: `handle_plugin_crash`
    (`zellij-server` `wasm_bridge.rs`) only paints a panic indicator, it does
    not remove the pane or unregister the plugin, so a crashed sidebar is
    serialized into the layout dump with its `plugin location` exactly as it
    appears in the manifest. A retrofit into such a tab therefore aborts on
    the dump (it already carries a sidebar) before installing any rail, so a
    live rail is never seated beside the dead one and the cleanup never
    observes two sidebars — no manifest liveness signal is needed.
35. **Swap re-seat flattens nested percentage regions when the available
    width changes.** Applying a swap layout re-seats existing panes by
    resolving the layout to absolute leaf geometry for the current free space
    (`LayoutApplier::apply_tiled_panes_layout_to_existing_panes` →
    `flatten_layout`, `layout_applier.rs:160,357`), not by re-applying the
    layout tree. When the tab's free space changes — here the docked↔undocked
    rail-width flip (28↔1) widens the region — `position_panes_in_space`'s
    percentage-constraint solve can fail, and `flatten_layout`'s `.or_else`
    branch then re-positions **ignoring the percentage sizes** (zellij's own
    comment: "a hack around some issues with the constraint system that should
    be addressed in a systemic manner", `layout_applier.rs:370-388`). A
    deeply-nested percentage region (`pane 50% { pane 50%; pane 50% } +
    pane 50%`) then collapses from nested (a column of two rows beside a
    column) to a flat side-by-side split on the wider swap — panes and content
    all survive, only the split nesting is lost. The split-preserving transform
    is innocent: it emits a byte-identical nested region in both swaps,
    differing only by rail width (verified offline). The loss is entirely in
    zellij's re-seat, and no template shape survives it — `flatten_layout`
    discards the tree, and the failing step is the percentage solve, not the
    structure. Still present on zellij main (post-0.44.3). Accepted as the
    split-preserving fidelity ceiling; upstream family: #1825, #2829, #1758,
    #4647.

## Historical v2 rebuild guidance

The following recommendations describe how to rebuild the shipped WASM
prototype within its old per-tab boundary. They preserve its engineering
lessons but do not supersede the managed-view product architecture.

1. **Manifest-derived state machine.** One `State` struct recomputed from
   `PaneUpdate`/`TabUpdate`; the only persistent fields are user intent
   (nav mode, an in-flight toggle's deferred swap step). All decisions as
   pure functions (`decide_toggle` + tests worked well — extend the
   pattern).
2. **Two-phase commands.** Event/pipe handlers only *record* intended side
   effects; a drain step executes them from a safe context, never calling
   response-reading shims from `pipe()`/`load()`. No `unwrap` anywhere near
   shim responses.
3. **Layout-first placement inside a prototype-owned tab.** The sidebar pane
   lives in that tab's layout, with docked and undocked (`size=1` sliver)
   `swap_tiled_layout` states; toggling steers swap layouts by name (see #29).
   The old first-toggle retrofit regenerated a damaged or foreign tab's swap
   set around its current arrangement (`docs/docking-approach.md`, Toggle v3),
   but that experiment is historical and must not become a product entry path.
   Create the managed tab with `new-tab --layout zaphod` instead. Its complete
   KDL includes chrome, the main region, and both swaps. Nothing is spawned,
   hidden, or shown during a steady-state toggle. Accept the prototype's
   fidelity finding: a deeply nested percentage region can flatten on
   collapse because Zellij's swap re-seat discards the layout tree when the
   percentage-constraint solve fails at the changed width (see #35). Panes and
   content survive; only the split nesting is lost.
4. **Keybind = pipe toggle only**, with `floating true` + `skip_cache` (dev)
   + an identity config key. Treat any pane-less instance as dead weight to
   be starved, not managed.
5. **Permission flow**: request on first render, stay selectable until
   granted, then lock focusability. Ship a `permissions.kdl` pre-grant
   snippet for headless testing.
6. **Test pyramid**: pure-function unit tests (row building, click mapping,
   toggle decisions, selection movement) + a scripted headless bench session
   for integration (spawn, toggle, click coordinates via simulated pipes).
