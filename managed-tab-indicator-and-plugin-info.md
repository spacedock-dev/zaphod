---
title: Managed-tab indicator and plugin provenance hotkey
status: ideation
source: captain direction 2026-07-13
sprint:
group: managed-tab-ux
sprint-readiness: defer
score: 0.7
started: 2026-07-13T12:30:05Z
completed:
verdict:
worktree:
issue:
pr:
mod-block:
id: jrp6se4epjqnnbr6vc29tazm
---

## Problem

Operators cannot tell at a glance whether a Zellij tab is a properly managed
Zaphod tab, and they cannot inspect the provenance of the rail actually loaded
in that tab. A sidebar-shaped or same-WASM lookalike must not be confused with
a tab that satisfies the managed-tab ownership proof.

The obvious implementation--renaming a tab and later using Zellij's undo--is
not reversible. In a disposable Zellij 0.44.3 session, stable tab `0` was
renamed `operator title`, then `🛰 operator title`; native
`undo-rename-tab -t 0` returned `Tab #1`, not `operator title`. `TabInfo`
exposes only the current name and no writer attribution. A design that stores
no separate base title therefore cannot distinguish its own decoration from an
operator rename after reload or proof loss.

## Proposed approach

### Put the smallest native invalidator first

The riskiest real Zellij capability is not drawing an emoji. It is retaining a
per-rail recovery record across a WASM reload while the native tab name remains
freely operator-editable. Two disposable tmux-hosted Zellij 0.44.3 probes were
run before choosing the action or lifecycle:

1. Stable-ID tab rename worked, but native undo reset the name to `Tab #1` and
   lost the operator title. The implementation must never use
   `undo_rename_tab` as its restoration mechanism.
2. A floating candidate rail was renamed to
   `zaphod-title-v1:operator-title`; `start-or-reload-plugin` retained plugin
   ID `4`, exactly one resident, and that exact pane title. Both probes used
   temporary config, data, socket, HOME, tmux server, and Zellij session roots,
   then removed them.

Implementation starts by turning probe 2 into the smallest disposable
regression: write a presentation record into the borderless rail pane title,
reload that exact plugin, and require the same plugin ID and record afterward.
If it fails for the integrated candidate, stop. Do not fall back to stripping
an emoji from an unattributed tab name, a file registry, native undo, or a
controller.

This task depends on, but does not assume the landing of, `kj`
(`managed-tab-safety-session-integration`). Begin implementation from the
integrated `kj` head and preserve its V3 `ManagedRailProof`--the exact
`zaphod_managed_tab "v1"` configuration plus equality between configured and
observed canonical WASM URLs--and its fresh PaneUpdate-to-TabUpdate stable-ID
mapping. If `kj` has not landed or those contracts differ, return to ideation
instead of recreating them.

### Reversible title state

Choose the exact managed indicator `🧭` and render the native tab name as
`🧭 {operator_title}`. The prefix is presentation only. Neither it, the tab
name, stable ID, CWD, geometry, pane title, nor a URL substring may satisfy or
repair `ManagedRailProof`.

Add a pure `reconcile_managed_title` transition around three inputs: the V3
proof, the stable tab ID/name resolved through `own_tab_position` plus kj's
fresh `observe_agent_tab_update` mapping, and a decoded presentation record.
The record contains a format version, stable tab ID, the exact operator title,
and the last exact decorated title. Encode it in the already borderless,
non-selectable rail pane title with an unambiguous versioned prefix. It is not
an ownership marker; read or write it only after `ManagedRailProof` succeeds.

The transition is ordered so a crash cannot destroy an operator title:

1. On first proof, take the complete observed tab name as `operator_title`.
   Write the recovery record with `rename_plugin_pane`, wait for PaneUpdate to
   echo that exact record, then rename the stable tab ID to the decorated name.
2. If the observed name equals `last_decorated`, do nothing. Repeated events
   and reloads therefore produce one prefix, not a growing prefix chain.
3. If proof still holds and the observed name differs from `last_decorated`,
   treat the complete observed value as a new operator title--never strip a
   prefix heuristically. Persist the new record first, then decorate it once.
4. Tab reorder changes only display position; the record and rename target use
   the stable ID. A same-WASM or same-shape rail without full V3 proof never
   reads a record or renames a tab.
5. On observed proof loss, restore `operator_title` only when the current name
   still equals `last_decorated`. If it differs, an operator/external writer
   won the race, so preserve the current name byte-for-byte. Clear the record
   only after the restoration or preservation is observed. The task's own
   close-self path performs this release before closing; closing the whole tab
   needs no rename because the title ceases to exist.
6. A missing, corrupt, wrong-tab, or unconfirmed record authorizes no title
   rewrite. Report the state in debug evidence and fail closed. Never guess a
   base title by removing `🧭`.

This state machine preserves an operator title that contains other emoji or
even the literal managed prefix: it regards the full observed rename as data.
The native pane-title recovery record is session-local presentation state and
must not be written to Zellij config, layouts, the permission cache, or a
Zaphod ownership registry.

### Runtime provenance action

Do not claim a new session-global chord is non-conflicting. Reuse the rail's
existing navigation entry (`Alt .`) and handle `i` only while that exact rail
owns focus in navigation mode; add a clickable `ⓘ` header target for the same
action. This is a runtime-scoped `Alt .`, then `i` route, not a persistent
Zellij keybind, and it requires no `reconfigure` change.

Before entering the info view, re-evaluate `ManagedRailProof`, require the
active stable tab to equal this rail's mapped tab, and require the tiled
resident. Extend the same V3 proof and fresh-active-tab inputs used by
`should_route_toggle_to_self` and `should_accept_observed_toggle_pipe`; do not
create a weaker info-specific ownership helper. An absent or lookalike proof
renders `Not managed` only in the rail receiving the local action and reveals
no claimed provenance; a stale cross-tab action is inert.

The smallest legible surface is a dismissible in-rail `PROVENANCE` view. It
wraps, without eliding characters, these two values:

- `loaded:` the exact observed `PaneInfo.plugin_url` (`file:` URL/path), after
  its equality with V3's configured canonical URL has been proved;
- `build:` an immutable string compiled into this WASM, formatted as package
  version plus full source revision and an explicit dirty/reproducible marker.

Stamp the build value at compile time through the normal artifact build and
reference it from WASM code. Runtime code must not inspect the filename, run
`git`, read the checkout, or infer a version from the loaded path. `Esc`
returns to the prior rail view and focus. The info string and title remain
presentation only and never become inputs to toggle, session delivery, or
gate admission.

## Acceptance criteria

### Offline (agent-reproducible)

**AC-O1 -- A managed title is reversible and idempotent against independent
native names.** A disposable Zellij 0.44.3 session starts with operator title
`captain 🧪 work`, observes one fully proved managed rail, and settles on
exactly `🧭 captain 🧪 work`. Native `rename-tab-by-id` changes it to
`review 🔧`; the rail settles on exactly `🧭 review 🔧`. Reorder and
`start-or-reload-plugin` leave the same stable tab ID, one prefix, and the same
recovery record. Forced proof loss restores exactly `review 🔧`, while a
rename racing proof loss is preserved byte-for-byte.

Verified by: native `list-tabs --json`, `list-panes --json --all --tab`, the
stable IDs, and before/after names supplied by the shell fixture rather than
the Rust assertions. The harness repeats reconciliation and reload at least
twice, and separately corrupts the record to prove no title write occurs.

**AC-O2 -- Lookalikes never gain the indicator.** Beside the managed tab, the
same disposable session creates an ordinary tab and a same-WASM, 28-column,
tiled rail with the same visible title but without one V3 declaration. Across
activation, rename, reorder, reload, and a stale runtime route, their native
tab names remain byte-identical and contain no Zaphod-added prefix.

Verified by: independent pre/post `list-tabs` and `list-panes` projections for
all three stable tab IDs. The negative rail matches the managed URL and visual
shape; only the missing full proof differs.

**AC-O3 -- The active managed rail reports artifact provenance, and only it
does.** From the managed tab, literal `Alt .`, then `i` displays the complete
canonical `file:` URL observed in native pane state and the build stamp known
when the candidate WASM was built. The URL remains character-for-character
equal after line wrapping. From the lookalike and ordinary tabs, the same
sequence either says `Not managed` in the locally focused rail or changes no
screen/native state; it never attributes the managed artifact to them.

Verified by: a candidate built with an externally supplied fixture revision,
the artifact's runtime render, native `plugin_url`, and tmux screen capture
reassembled along the renderer's declared wrap width. A second build with a
different fixture revision must change the displayed `build:` value while its
filename remains the same; renaming the file must not change the stamp.

**AC-O4 -- The feature adds no authorization or standing-state mutation.**
Existing V3 route-offer and receipt negatives remain green for every missing
or mismatched proof input. Title/info code never feeds those decisions. The
disposable journey leaves operator config, layouts, permission cache, data
root, and current sessions at their pre-test hashes/inventory; it adds no
persistent `i` or new global keybind.

Verified by: focused Rust decision tests, repo path-diff inspection, and the
isolated harness's external before/after hashes plus post-cleanup socket,
session, tmux-server, and temporary-root absence checks.

**AC-O5 -- README gives the exact visible contract and deferrals.** README
states what `🧭` means, the `Alt .`, then `i` journey, the two displayed
provenance fields, rename/reload restoration, and the fact that presentation
is not ownership. It explicitly says gate notification and decision-ledger
projection are deferred.

Verified by: captain review of the concrete documentation change below
against the live candidate after AC-O1 through AC-O4, not by a substring test
over README.

### Captain-live (only after AC-O1 through AC-O5)

**AC-I1 -- The captain can distinguish and inspect the real managed tab
without risking standing state.** In the isolated profile, the captain sees
one `🧭` tab and one same-WASM lookalike without it, renames the managed tab,
reloads the rail, and sees one indicator with the rename intact. `Alt .`, then
`i` shows the exact candidate path and build stamp; the lookalike does not.

Verified by: the captain's observation plus captured stable tab/plugin IDs,
native names/URLs, build fixture value, and pre/post standing-state hashes.
The drill does not use `WORK` or the operator's normal config/layout/data root.

## Test plan

1. First rerun the disposable native recovery probe on the integrated
   candidate: write a rail-pane presentation record, reload the exact URL, and
   require the same pane ID/title. Also retain the negative native-undo result.
   Failure invalidates the title direction before product code or a hotkey is
   added.
2. Add red pure tests for `reconcile_managed_title`: acquire, record-before-
   rename, duplicate events, operator rename, prefix-containing operator text,
   reorder, reload recovery, proof loss, racing rename, corrupt record, and
   teardown. Extend V3's `ManagedRailProof` and kj's stable mapping; do not
   duplicate them.
3. Add a deterministic build-stamp fixture and pure info-action tests. Prove
   exact observed URL plus compiled stamp on the positive path and `Not
   managed`/no-op for missing marker, URL mismatch, stale active tab, floating
   rail, corrupt mapping, and lookalike.
4. Extend the integrated disposable tmux smoke with the three-tab journey from
   AC-O1 through AC-O4. Use native single-line JSON tab/pane projections and
   literal key bytes; never activate standing config or the parked profile.
5. Run focused and repository checks: `cargo test -q`, `cargo check --tests`,
   `go test ./...` in `grout`, `tests/build-artifact-test.sh`,
   `tests/zellij-new-tab-test.sh`, the integrated kj smoke, and
   `git diff --check`. No Zellij-dependent check may silently skip.
6. Only after offline proof passes, run AC-I1 in the same disposable profile.
   A recovery-record, exact-path, or lookalike failure returns to ideation; it
   does not authorize a registry, controller, or standing-state edit.

## Concrete README change

In `README.md` immediately after “Create a fresh managed tab,” add a
“Managed title and provenance” subsection with this content:

> A tab whose resident rail satisfies the V3 marker plus exact loaded-WASM
> proof is shown as `🧭 <your title>`. Zaphod preserves your complete tab
> title across renames and rail reloads; the emoji and title are display only,
> never ownership or authorization. A same-shape or same-WASM tab without the
> full proof is unmarked.
>
> In a docked managed rail, press `Alt .`, then `i` (or click `ⓘ`) to open
> `PROVENANCE`. It shows the exact loaded `file:` URL/path and the version/build
> stamped into that WASM. `Esc` returns to the rail. This adds no persistent
> keybind and does not derive a version from the filename or checkout.
>
> Gate notification and decision-ledger projection are deferred. This feature
> neither discovers or delivers gates nor adds a gate state to the tab title.

Also add the title/rename/reload/provenance journey's disposable smoke command
beside the existing real-key smoke packet. Do not rewrite historical prototype
documents or claim `kj` behavior before its integration lands.

## Out of scope

- Implementing, discovering, delivering, or acknowledging gate notifications;
  projecting gate decisions or any ledger into the rail; or choosing a future
  gate-state emoji. The presentation record is versioned so a later task can
  add a distinct state, but this task implements only `managed`.
- Treating title, emoji, recovery metadata, stable ID, CWD, shape, filename, or
  URL substring as managed ownership; weakening either V3 decision point or
  kj's recipient proof.
- A title registry, controller, helper pane/process, new sidecar lifecycle,
  custom tab bar, persistent/global provenance keybind, or writes to standing
  Zellij config/layout/data/permission state.
- Moving tabs across sessions, repairing deferred complex-layout `Alt /`,
  second-client routing, or changing session-row delivery.

## Stage Report: ideation

- DONE: Prove the managed-proof-to-title indicator can be idempotent without destroying operator renames, and put the smallest native invalidation probe first.
  Disposable Zellij 0.44.3 probes refuted native undo (`Tab #1`, not the operator title) and proved the same plugin ID/pane-title recovery record survives reload; the record-first state machine and AC-O1/O2 cover rename, reorder, reload, proof loss, races, corruption, and teardown.
- DONE: Define a runtime-scoped info action that reports the exact loaded WASM path plus an artifact-stamped version and fails closed for lookalikes.
  “Runtime provenance action” chooses conflict-free rail-local `Alt .`, then `i`/`ⓘ`, exact observed `PaneInfo.plugin_url`, a compiled build stamp, and full V3/fresh-active-tab revalidation; AC-O3 independently measures both builds and the lookalike negative.
- DONE: Write measurable offline and captain-live acceptance criteria, the concrete README change, and an explicit deferral for gate notification and ledger projection.
  AC-O1 through AC-O5 and AC-I1 use native IDs/names/URLs, external build fixtures, screen output, hashes, and cleanup state; the README text and Out of scope section explicitly defer gate notification and decision-ledger projection.

### Summary

Ideation chose `🧭` and a reversible, session-local rail-pane recovery record after the smallest native probes showed that Zellij reload preserves that record but native tab undo destroys the operator title. The provenance action stays inside the existing rail navigation route, reports the exact loaded URL and compiled artifact identity, and remains inert without the complete V3/kj proof; no product code or standing Zellij state was changed.
