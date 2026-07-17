---
id: tjj3aqdq4c4at5wrke8cmk0v
title: Keep the managed rail at one fixed width without blocking native fullscreen
status: implementation
source: captain direction after live dirty-tab layout corruption investigation, 2026-07-17
sprint: s1-managed-tab-safety
group: layout-stability
sprint-readiness: ready
started: 2026-07-17T10:10:10Z
completed:
verdict:
score: 0.98
worktree: .worktrees/spacedock-ensign-fixed-width-managed-rail
issue:
pr:
mod-block:
---

## Problem

The managed rail currently toggles between fixed widths by replacing or cycling the whole tiled layout. In tabs with additional panes, that operation can flatten or stack the user layout and misplace tab-bar/status-bar chrome. The first safe product step is to keep the rail docked at one fixed expanded width, make toggle input incapable of restructuring the tab, and preserve Zellij native fullscreen for ordinary and rail panes.

## Captain constraint

Prefer layout preservation over reclaiming the rail columns. Assume native pane fullscreen remains supported, and verify that assumption as part of the design and live acceptance proof.

## Proposed approach

Keep one layout-owned, tiled rail at `size=28` for the lifetime of a managed tab. Remove the `docked` and `undocked` swap layouts from `layouts/zaphod.kdl`; the birth layout retains exactly one tab-bar row, the 28-column rail, one `children` content region, and one status-bar row. Adding, splitting, resizing, floating, or fullscreening content panes must use Zellij's ordinary pane operations and must never cause Zaphod to apply or cycle a layout.

Retire the dock toggle rather than assign its keys a surprising new meaning:

- Persistent `Alt /` remains a silent `NoOp`. The rail stops requesting a runtime `MessagePluginId` binding, drops `Reconfigure` from `permissions_for_config`, and treats any stale or directly sent `toggle` pipe as an unknown inert message.
- Replace the header's `⇄` affordance with the static word `FIXED`. Clicking the header does nothing; pane, session, and gate rows keep their existing actions.
- Fullscreen remains Zellij-native. Zaphod adds no fullscreen keybinding: an ordinary pane uses the operator's normal fullscreen action, while the rail can be focused through its existing navigation mode and then fullscreened with the same native action. Exiting fullscreen restores the fixed rail and prior content geometry.

Implementation should extend the existing pure seams `permissions_for_config` and `decide_rail_click`: the former no longer emits `Reconfigure`, and the latter maps the header to `ClickAction::None`. Remove the now-unreachable `decide_toggle`, runtime-route, pending-steer, dump-transform, `override_layout`, and swap-steering machinery instead of retaining a second layout-mutating path. Keep `decide_rail_click`'s row behavior and the agent/session/gate delivery functions unchanged.

## Acceptance criteria

### Offline

- **O1 — A populated managed tab has one stable fixed rail and intact chrome.** In an isolated 160x48 tmux-hosted Zellij 0.44.3 session, a selected-checkout managed tab with two added terminal panes has exactly one tiled, non-suppressed Zaphod rail at `x=0`, `y=1`, `28x46`, one `160x1` tab bar at `y=0`, and one `160x1` status bar at `y=47`.
  **Verified by:** an automated smoke captures native `list-panes --json --all --command --geometry --state --tab`, identifies the rail by exact canonical WASM URL and stable tab ID, and compares the observed six-pane inventory with these independently specified dimensions.
- **O2 — Former toggle inputs cannot mutate pane layout.** Literal `Alt /`, a real click on the `FIXED` header, and a direct stale `toggle` pipe leave pane IDs, plugin URLs, terminal commands, pane geometry, chrome geometry, and normalized `dump-layout` state unchanged; the rail remains 28 columns.
  **Verified by:** the isolated tmux smoke records before/after native inventories and normalized layout dumps for each input, while Rust tests drive `permissions_for_config` and `decide_rail_click` and require `Alt /` route absence plus an inert header result.
- **O3 — Native fullscreen round-trips both pane kinds without layout loss.** With the same two extra panes present, fullscreening an ordinary terminal and the Zaphod rail in turn expands the target to the full `160x46` content rectangle; leaving fullscreen restores every non-focus field from O1, including the rail at 28 columns and both chrome rows.
  **Verified by:** the smoke uses Zellij's pane-ID-targeted native `toggle-fullscreen` action for each target, observes `is_fullscreen=true` and full content geometry, then compares canonical post-round-trip snapshots against the pre-fullscreen baseline.
- **O4 — Entry, row interaction, and permission behavior regress neither functionality nor standing configuration.** Managed entry still creates one tab from the selected checkout, row clicks still focus their exact targets, subscriber delivery still reaches only its stable-tab/token-bound rail, and no test writes standing config, layout, data, socket, or permission roots.
  **Verified by:** `cargo test`, `tests/zellij-new-tab-test.sh`, the existing recipient smoke, and the revised isolated tmux smoke all pass with standing-root hashes unchanged; the permission fixture expects no `Reconfigure` grant.

### Interactive

- **I1 — Extra panes remain arranged after former toggle inputs.** In the captain's `WORK` session, create a fresh managed tab, add one vertical and one horizontal split, note their geometry plus both chrome rows, press literal `Alt /`, and click the `FIXED` header; nothing moves, stacks, disappears, or changes width.
  **Verified by:** captain observation plus before/after `list-panes --json --all --geometry --state --tab` and `dump-layout` captures reviewed at the implementation gate.
- **I2 — The captain's native fullscreen journey works for ordinary and rail panes.** Fullscreen and restore an ordinary pane with the captain's existing Zellij binding; then enter rail navigation, fullscreen and restore the rail with that same native binding, and exit navigation.
  **Verified by:** captain confirms both targets occupy the content area while fullscreen and the original split/chrome geometry returns after each restore, with no permission prompt or new pane.

## Test plan

1. **Riskiest mechanism first — already exercised:** in a disposable 160x48 tmux + Zellij 0.44.3 session, birth a 28-column fixed, layout-owned plugin rail with canonical chrome, add two terminals, and fullscreen/restore one ordinary pane and the rail. The terminal and plugin each reached `160x46`; the normalized six-pane snapshots after both round-trips matched the baseline, with chrome still at `y=0`/`y=47` and the rail restored to `28x46`. A built-in tiled plugin isolated this host-layout assumption from Zaphod permission behavior; O3 repeats it with the shipped WASM. The disposable server, socket, data root, and tmux server were removed.
2. Add failing pure tests for permissions and header click, then remove route/toggle/swap code and reduce the layout to the fixed base.
3. Revise the real-key smoke: add two panes before baseline; prove literal `Alt /`, header click, and stale pipe are no-ops; perform both pane-ID fullscreen round-trips; compare native geometry, identity, layout, and chrome after every operation.
4. Run the unit, layout/entry, recipient, and tmux smoke suites in isolated roots. Only after all offline checks pass, run I1 and I2 in the captain's `WORK` session.

## Required documentation diff

- `README.md`: replace the screenshot's `⇄` with `FIXED`; replace the dock/sliver feature with the always-28-column behavior, silent `Alt /`, inert header, and native-fullscreen escape hatch; remove the runtime-route/Reconfigure instructions; revise live verification and Status accordingly.
- `docs/roadmap.md` and `docs/zaphod-workspace-architecture.md`: change Sprint 1's current managed-tab toggle claims to a fixed layout-owned rail with layout-inert former toggle inputs. Leave future driver-level resizing explicitly dependent on a safe upstream API rather than presenting it as shipped.
- `SPEC.md`: mark the docked/sliver and regenerated-swap passages as historical prototype behavior, record the fixed-rail safety boundary and fullscreen proof, and retain the old failure analysis as evidence.
- `layouts/zaphod.kdl` and smoke-harness documentation: describe the single fixed layout and the new no-op/fullscreen proofs; do not claim that any key cycles a swap layout.

## Out of scope

- Reclaiming rail columns through resize, swap layouts, `override_layout`, hide/show, float/embed, or another automatic reflow.
- Rebinding `Alt /` to fullscreen or another global action; the operator's existing native fullscreen binding remains authoritative.
- Hot-repairing tabs created by an older docked/sliver WASM or layout; create a fresh managed tab after upgrading.
- General retained-pane layout transactions, exact fixed-dimension setters, right-docking APIs, tmux parity, or changes to future hub/adoption architecture.

## Stage Report: ideation

- DONE: Define the fixed-width rail behavior, including the exact inert/replacement behavior for Alt / and the header toggle, without any whole-tab override or swap-layout mutation; include the required user-facing documentation diff.
  The proposed approach fixes the rail at 28 columns, leaves `Alt /` as silent `NoOp`, replaces `⇄` with inert `FIXED`, removes all toggle layout APIs, and names the README/roadmap/architecture/SPEC diff.
- DONE: Specify independently reproducible offline and captain-driven interactive acceptance proofs that extra panes retain geometry/chrome and that native fullscreen still works for ordinary and rail panes.
  O1-O4 define isolated native-state proofs; I1-I2 require captain observation and before/after native inventories in `WORK`.
- DONE: Exercise the riskiest assumption first in a throwaway Zellij session: a managed fixed-width rail remains fullscreen-compatible after the tab gains additional panes, or record the concrete invalidating result.
  Zellij 0.44.3 at 160x48 passed ordinary and fixed-plugin fullscreen round-trips after two added terminals; normalized six-pane geometry/chrome snapshots returned exactly to baseline.

### Summary

The design removes layout mutation from the rail's steady state: one 28-column birth-layout rail, inert former toggle inputs, and native fullscreen as the temporary space-reclamation mechanism. A disposable host-level probe validated the riskiest fullscreen assumption, while the implementation acceptance proof intentionally repeats it with the real Zaphod WASM and captain interaction.
