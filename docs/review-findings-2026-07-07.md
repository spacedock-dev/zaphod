# Code review findings — 2026-07-07 (Fable review @ bce89a4)

Fresh-eyes review of `src/main.rs` after the v3.1–v3.12 arc. Tests 95/95, `cargo check --tests` clean. Findings ranked; F1 mechanically verified (pure fns run in a scratch harness). Fix order: F1 first, then F2–F4 together, then triage the rest.

## Correctness

### F1 — HIGH, CONFIRMED — seeder-abort misses single-line rails
`block_has_sidebar` (`src/main.rs:1131-1139`) recognizes a rail only via `plugin_location` (needs a bare `plugin` first line) and gates recursion on `block.len() >= 2`. A **single-line** rail — `pane name="sidebar" size=28 borderless=true { plugin location="…zellij-sidebar.wasm" { rail "1" } }`, the exact shape zellij dumps (live tab-7 fixture `main.rs:3114`) — is not detected → `dump_contains_sidebar` returns false → the seeder-abort (`main.rs:612`) misses it. This is the **same single-line class `bce89a4` fixed in the transform** (`inline_plugin_location`, `main.rs:1205`); the detector's twin was never fixed. **Impact:** under the relaxed election (v3.8) + PaneUpdate lag, instance B dumps a tab already retrofitted by A (A's rail single-line in the dump), the abort misses it → B fires a second override on the already-docked tab (re-absorb flash, swap-set replace, steer/cooldown churn) — reopening the v3.7 double-override class and weakening the SPEC #34 "aborts on the dump" argument. **Likely a real contributor to the collapse-reopens/re-stack flakiness seen across drills, and it makes the freshly-written docs' "dump-abort dedups" claim not-yet-true.** Fix: `block_has_sidebar` leaf case consults `inline_plugin_location(&block[0])` + classify, mirroring `extract_chrome_panes`. TDD: feed the live tab-7 dump to `dump_contains_sidebar` (must return true); existing test `main.rs:3333` uses only multi-line fixtures (the test-realism gap again). ~2 lines.

### F2 — MEDIUM, CONFIRMED-by-trace — clean-tab steer blindly cycles foreign swap sets
`main.rs:732-736`: a tiled resident on a clean tab gets `SteerSwap{backwards:true}` for ANY name ≠ "docked", including provably-foreign ("vertical"/"stacked"). But `pending_steer_disposition` (`main.rs:841`) treats a foreign name as Drop — the two ends disagree. A tiled sidebar on a tab carrying a builtin/captured swap set cycles foreign templates, re-tiling the user's panes while the dock never toggles. Fix: route names outside `{BASE, docked, undocked, None}` to `RegenerateSwaps` (installs our set) instead of a blind steer.

### F3 — MEDIUM, SUSPECTED — immediate toggle path not gated on floating-visible
The deferred steer waits for floats to hide (`main.rs:833-835`) but the immediate `SteerSwap` in `perform_toggle` has no `floating_visible` gate. Per the TabState comment (`main.rs:139-142`), `swap_name`/`swap_dirty` speak for the FLOATING layer while floats are visible, and next/previous_swap_layout swap whichever layer is visible → pressing Alt-/ with a floating pane up cycles the floating swap set and reads the wrong layer's dirty flag. `floating_visible` is already in TabState, just not passed to `decide_toggle`.

### F4 — REFUTED 2026-07-07 (was MEDIUM-LOW, CONFIRMED) — non-tab-bar chrome always re-emitted at bottom
`is_top_bar` (`main.rs:1093-1099`) matches only "zellij:tab-bar"; all other `zellij:*` chrome lands in `chrome_bottom` — the routing is real, but the bug premise ("compact-bar is a TOP bar") is false: `zellij setup --dump-layout compact` (0.44.1) places compact-bar BELOW the region pane, and the compact swap layouts' `tab_template "ui"` agrees (`children`, then compact-bar). tab-bar is the only top bar zellij ships, so the name-based routing already re-emits every builtin layout's chrome at its correct home; the proposed one-liner would have INTRODUCED a relocation bug for builtin-compact users. Residual (possible future respec): placement is canonicalized by plugin name, so CUSTOM chrome placement (e.g. a user's top-placed status-bar) is normalized on retrofit — a faithful fix is position-preserving extraction (record above/below the first region pane), a design change affecting custom-placement users only.

### F5 — LOW, SUSPECTED — fallback_swap_cycle blind-cycles, dead from undocked
`main.rs:940-943`: two blind `next` calls violate SPEC #29. From "undocked" (last entry): call 1 snap-folds (damage latch, no advance), call 2 is next-past-end → resets to 0 without applying → dead press; next press wraps back to undocked = another dead press (~3 presses to toggle). Only reachable on the degraded path (no grant / stale tab_states / dump error), but it's the "never blind-cycle" law broken in the code's own fallback.

### F6 — LOW, SUSPECTED — dump and override can aim at different tabs
`install_split_preserving_swaps` dumps by the plugin's believed stable tab id (`main.rs:601` via `rebuild_target`), while `override_layout` applies to the server-resolved active tab at processing time (`main.rs:634-640`). Divergence windows: `active_tab_for_decision` cached-position fallback (`main.rs:1411-1425`); user switches tabs in the dump→override dispatch gap (override runs on a spawned server thread). Then tab B is absorbed under swaps built from tab A → unfittable → B's splits stay stacked until a later regenerate repairs it; the seeder-abort also checked the wrong tab. Relaxed election multiplies stale actors.

### F7 — LOW/EDGE — armed steer + floating-visible Keep is unbounded
`main.rs:833-835` keeps the press armed while floats are visible; `should_swallow_toggle` (`main.rs:914-927`) eats all same-tab presses meanwhile → showing floats after a regenerate press then hiding them minutes later triggers a surprise collapse. Bound via cooldown expiry, or let a fresh same-tab press supersede rather than be swallowed.

### F8 — LOW/EDGE — clicking a row during nav mode leaves nav stuck on
`handle_click` `FocusPane` (`main.rs:472-478`) focuses the pane but never exits nav: `nav_mode` stays true, the pane stays selectable, the focus-handback guard stays disabled, the inverse highlight persists, and Enter/Esc route to the now-focused terminal. Fix: `ClickAction::FocusPane` should `exit_nav(false)` when `nav_mode`.

### F9 — MEDIUM, CONFIRMED-by-trace — builtin non-bar plugins classified as chrome, pane destroyed
Found 2026-07-07 while refuting F4 (line refs @ 9b552bd). `classify_plugin_location` (`main.rs:1312`) treats EVERY `zellij:*` location as Chrome, but strider is a 20%-wide REGION pane, not a bar (`zellij setup --dump-layout strider`): on retrofit `extract_chrome_panes` strips a builtin-strider user's strider pane and re-emits it as a `size=1 borderless` bottom row — the file browser is destroyed, and every regeneration re-destroys it. Fix: only genuine bars are Chrome (tab-bar, status-bar, compact-bar); other `zellij:*` panes ride along like third-party region panes. Check 0.44.1's builtin plugin set for other real bars before choosing allowlist vs denylist.

### F10 — LOW, CONFIRMED — chrome re-emitted at hardcoded size=1
Found 2026-07-07 while refuting F4 (line refs @ 9b552bd). `chrome_row` (`main.rs:1106`) emits every extracted chrome row as `size=1`, but strider's builtin status-bar is `size=2` (`zellij setup --dump-layout strider`) — chrome height fidelity is lost on retrofit. Fix: carry the extracted pane's size prop into the re-emitted row.

### Minor race
Before the redundant-rail cleanup fires, two tiled rails in one tab both satisfy the resident branch and both steer on one press — a double-step that can enter the unreliable next-past-end zone; self-heals when the higher-id rail closes (`main.rs:980-993`).

## Cleanups / dead code (lower priority)
- `rail_pane_kdl` (`main.rs:1023-1035`) interpolates config values + plugin URL raw into KDL → a value containing `"` yields malformed layout + silent override no-op (compounds F5). Add escaping.
- `row_marker` (`main.rs:1485`) is a one-line delegate to `state_marker` — collapse.
- `sidebar_instances` sorts by pane_id (`main.rs:1447`) but every consumer is order-independent — dead weight.
- `AgentState::Done` (`agent.rs:16-17`) `#[allow(dead_code)]`, no producer — YAGNI.
- Chrome removal from a 3+-child percent container leaves siblings summing <100% (e.g. 33+33) — unverified zellij accepts it; add a fixture.
- Comment-convention sweep: temporal "drill"/history narration (`main.rs:700-703, 757-769, 1880-1887, 1899-1905`) violates the repo's evergreen-comment rule — deliberate (rationale is useful), not blind.
- `rows_for_own_tab` (`main.rs:1518-1538`) includes floating terminal panes (no `is_floating` filter) — looks intentional but undocumented; add a comment.

## Clean areas (checked, no findings)
Poll gate / backoff / pre-grant panic safety (transitively airtight — PaneUpdate/TabUpdate gated on ReadApplicationState); bootstrap parked press (unpark events are permission-gated, one-shot); focus-handback crash guard; close_self one-shot; redundant-rail cleanup; KDL string machinery (prop_span/brace_delta quote/escape handling); no mocked-behavior tests.
