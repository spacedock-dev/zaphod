---
id: v3d4m3zfryre1eypjnaetd37
title: Alt slash only changes a Zaphod-created tab
status: implementation
source: staff review of fp merge 2026-07-12; captain direction
sprint: s1-managed-tab-safety
group: walking-skeleton
sprint-readiness: ready
started: 2026-07-12T14:50:58Z
completed:
verdict:
score: 1.0
worktree: .worktrees/spacedock-ensign-managed-tab-toggle-authorization
issue:
pr:
mod-block:
---

## Problem

The shipped entry proves that a visible, tiled resident can receive `Alt /`; it
does not prove that the resident came from Zaphod's managed layout. The route
checks accept a tiled active pane, and nearby sidebar classification recognizes
URLs by substring. Neither establishes layout provenance. A lookalike can
therefore install and consume the runtime route on its own tab. This is a
Sprint 1 safety gap in the existing entry journey, not Sprint 2 work.

## Required outcome

An operator can press `Alt /` in a fresh tab created by
`scripts/zellij-new-tab.sh` and change that tab's known rail state. In every
unmanaged, foreign, floating, absent, stale, or replaced-rail context, the key
is inert: no layout override, pane creation, focus change, or plugin reload.

For Sprint 1, “Zaphod-created” means a tab launched from the explicit managed
layout, not a visual resemblance. Copying that complete declared layout is an
explicit opt-in to the same behavior; this task does not claim to defend
against a user who deliberately forges that declaration. It must prevent an
ordinary same-WASM or filename lookalike from becoming eligible by accident.

## Proposed approach

Start with one isolated native-state spike. Render a disposable managed layout
whose rail carries the explicit configuration below, then create a second tab
with the same candidate WASM, `rail "1"`, sidebar name, and 28-column tiled
geometry but without these fields:

```kdl
zaphod_managed_tab "v1"
zaphod_wasm_url "__ZAPHOD_WASM__"
```

The spike passes only if the real Zellij `dump-layout` retains both fields on
the launched rail and the existing `PaneUpdate` reports an exact `plugin_url`
equal to `zaphod_wasm_url`. The current API supports this shape:
`Sidebar::load` already receives the plugin configuration map, Zellij 0.44.3
records it in `PluginInfo`, and the live `WORK` layout dump preserves the
existing `rail "1"` configuration. The spike nonetheless exercises the
actual `Alt Shift z` launch path before product code relies on the new fields.
If it fails, stop and report the negative evidence. Do not substitute a title,
CWD, tab position, URL substring, token file, lease, or controller.

If it passes, add those two fields to all three rail blocks in
`layouts/zaphod.kdl` (base, `docked`, and `undocked`). Treat their conjunction
with the exact manifest URL as `ManagedRailProof`. It is a declarative layout
provenance check, not a secret or registry.

Extend the existing pure route decisions, rather than adding a driver:

- `should_route_toggle_to_self` may call `reconfigure()` only when
  `ManagedRailProof`, permission, active-tab equality, tiled placement, and a
  not-yet-requested route all hold.
- `should_accept_observed_toggle_pipe` may call `perform_toggle()` only when
  the same `ManagedRailProof`, a keybind source, permission, active-tab
  equality, and tiled placement hold.
- No URL substring or `sidebar_instances()` result may establish this proof.
  Any remaining substring-based duplicate cleanup is not route authorization
  and stays outside this narrow change.

The entry script continues to render and validate the selected checkout's
layout, and persistent `Alt /` remains `NoOp`. Extend its layout validation
so every managed rail block has the marker and matching rendered URL. Keep the
tmux-hosted smoke and direct `Alt Shift z` entry; add no lease, custom PTY,
controller, public CLI, binding core, or Sprint 2 dependency.

## Acceptance criteria

### Offline

**AC-1 — The explicit layout proof survives a real fresh-tab launch.**
Verified by: the isolated tmux smoke creates the managed tab through literal
`Alt Shift z`; its native `dump-layout` contains `zaphod_managed_tab "v1"` and
the exact candidate `zaphod_wasm_url` on the active rail. Rendered-layout
validation confirms that the base, `docked`, and `undocked` rail blocks carry
the same fields. A same-WASM, tiled, 28-column lookalike lacks them while
matching the visual baseline.

**AC-2 — A managed tab remains usable.**
Verified by: after the ordinary pre-grant settlement, literal `Alt /` changes
the managed rail from 28 to 1 columns and changes its visible/native layout.
The candidate pane ID, command, tab ID, and focus projection remain unchanged.

**AC-3 — Both route offer and pipe receipt require managed proof.**
Verified by: focused Rust tests make `ManagedRailProof` false for each missing
or mismatched marker/URL input and prove both route helpers reject it, even
with permission, active-tab equality, tiled placement, and a keybind source.
The positive case requires the full proof; a URL substring, title, CWD,
geometry, or `rail "1"` alone fails.

**AC-4 — A same-WASM tiled lookalike is inert after a real managed route exists.**
Verified by: the smoke first proves the managed literal key path, then switches
the same client to the active lookalike. Before/after projections of that
tab's pane inventory, layout, focus, loaded-plugin IDs, and captured screen
are byte-identical after literal `Alt /`; the key creates no pane, reload, or
layout transition. This also exercises a stale runtime route aimed at the
former managed plugin.

### Interactive

**AC-I1 — The reported `WORK` behavior is settled on a live client.**
Verified by: after offline checks pass, the captain uses the current `WORK`
client to create a fresh tab through the entry command or `Alt Shift z`, grants
the normal permission if prompted, and presses `Alt /`. The new tab toggles;
an existing unmanaged or legacy tab remains unchanged. Record the client/tab
used and whether a current-client route reappears when that tab becomes
visible. A client-routing defect discovered here is reported separately unless
it invalidates `ManagedRailProof` itself.

## Test plan

1. Run the configuration-provenance spike first in the existing isolated tmux
   harness. It must show the two managed fields in the native dump for a
   literal-entry tab, omit them from the same-WASM lookalike, and retain the
   same visual state for both. Failure ends this task's implementation route;
   no fallback mechanism is invented.
2. Add red pure tests around a single `ManagedRailProof` helper and the two
   existing route helpers. Cover missing marker, wrong marker version, missing
   expected URL, exact-URL mismatch, floating placement, stale active tab,
   non-keybind pipes, and the fully positive case.
3. Add layout/helper tests that require every base and swap-layout rail to
   carry the marker and selected-checkout URL. Preserve the existing
   `zaphod_render_layout` and direct entry contract.
4. Extend `tests/zellij-tmux-smoke-test.sh` with the same-WASM, tiled,
   sidebar-shaped adversary. Use the returned tab ID and native state rather
   than a display name to select it. Compare the lookalike's pre/post native
   inventory, dump, focus projection, and screen after literal `Alt /`.
   Retain the existing disposable-root, tmux-server, session, and standing-root
   cleanup assertions; do not revive the parked profile harness.
5. Run the focused Rust tests, entry-script suite, and isolated tmux smoke
   before the captain's `WORK` drill. The captain drill is the only live proof
   and does not authorize a broad multi-client feature claim.

Doc diff proposed: update README's “Create a fresh managed tab” section to
define the explicit `v1` marker plus exact candidate URL as the `Alt /`
eligibility proof, and state that sidebar shape or filename alone never
enables the key. The command, keybindings, and upgrade path stay unchanged.

## Out of scope

Sprint 2 bb session ingestion and `zaphod subscribe`; the AWK
config-rewriter/concurrency finding; multi-client feature expansion; a
cryptographic or persistent per-tab registry; 7h, 4d, leases, custom PTYs,
controllers, binding cores, and public CLI work. This follow-up does not block
bb.

## Stage Report: ideation

- DONE: Define a real managed-tab authorization proof at route offer and receipt.
  `ManagedRailProof` combines explicit `zaphod_managed_tab "v1"`, exact rendered URL, and the existing permission/active/tiled/keybind guards at both decisions.
- DONE: Design the smallest tiled/sidebar-shaped unmanaged adversary.
  The isolated same-WASM rail retains `rail "1"`, sidebar naming, and 28-column tiled geometry but omits the two provenance fields; its unchanged native/screen fingerprint is the negative proof.
- DONE: Keep the direct fresh-tab entry and tmux smoke; exclude 7h machinery.
  The plan extends `Alt Shift z` and the disposable tmux harness only; it explicitly rejects a registry, lease, custom PTY, controller, or Sprint 2 dependency.

### Summary

The repair makes layout provenance explicit and checks it before either
installing or consuming the runtime route. A same-WASM lookalike becomes a
strong negative fixture without adding lifecycle state. The first isolated
configuration spike can still invalidate this design before code changes.
