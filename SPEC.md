# Zaphod — zellij pane-switcher sidebar

Spec and from-scratch learnings, distilled from the v1 prototype campaign
(zellij 0.44.1, 2026-06-10).

## What it is

A per-tab sidebar that answers "what is running in this tab and what does it
want from me?" — built for stacked-pane workflows where many Claude/codex
agents run side by side.

### Validated UX (what the prototype proved out with real use)

- **Left rail, ~28–30 cols**, listing the current tab's terminal panes,
  top-to-bottom.
- Per pane row (two lines):
  - markers: yellow `●` focused pane, red `●` agent working (viewport contains
    `esc to interrupt`), cyan title for agent panes (`✳` in title — Claude
    Code/codex set this)
  - dim second line: the pane's last non-empty terminal line (what the agent
    is asking/doing), polled every 2s via `get_pane_scrollback`
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
    it is retired too: the adopted architecture keeps a sidebar pane in every
    tab's layout and toggles by cycling swap layouts
    (`docs/docking-approach.md`), so nothing is shown, hidden, or spawned.
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
    damage there is invisible.
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

## If building v2 from scratch

1. **Manifest-derived state machine.** One `State` struct recomputed from
   `PaneUpdate`/`TabUpdate`; the only persistent fields are user intent
   (nav mode). All decisions as pure functions
   (`decide_toggle` + tests worked well — extend the pattern).
2. **Two-phase commands.** Event/pipe handlers only *record* intended side
   effects; a drain step executes them from a safe context, never calling
   response-reading shims from `pipe()`/`load()`. No `unwrap` anywhere near
   shim responses.
3. **Layout-first placement.** The sidebar pane lives in every toggled tab's
   layout, with docked and undocked (`size=1` sliver) `swap_tiled_layout`
   states; toggling cycles swap layouts only. The default layout stays
   chrome-only with an explicit `tab { pane }` (see #27); tabs get the
   sidebar + swap set from `new-tab --layout zaphod` at birth or from a
   one-time `override_layout` retrofit on first toggle — complete KDL:
   chrome, stacked main, both swaps (`docs/docking-approach.md`, Adopted
   architecture). Nothing is spawned, hidden, or shown.
4. **Keybind = pipe toggle only**, with `floating true` + `skip_cache` (dev)
   + an identity config key. Treat any pane-less instance as dead weight to
   be starved, not managed.
5. **Permission flow**: request on first render, stay selectable until
   granted, then lock focusability. Ship a `permissions.kdl` pre-grant
   snippet for headless testing.
6. **Test pyramid**: pure-function unit tests (row building, click mapping,
   toggle decisions, selection movement) + a scripted headless bench session
   for integration (spawn, toggle, click coordinates via simulated pipes).
